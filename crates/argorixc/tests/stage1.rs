//! ESP-014: the first self-hosted compiler.
//!
//! Stage0 builds stage1 from `compiler/main.argx` and the rest of `compiler/`
//! and `stdlib/`. Stage1 is then run on its own sources, as `argorix.build` at
//! the root of the repository describes them. It must reproduce, byte for
//! byte, the C stage0 wrote for it, and a manifest that describes the build
//! exactly. That C, compiled, must be a working compiler: it does the same
//! build again with the same results.
//!
//! The driver's other outcomes are checked against stage0's own command line:
//! the rendered diagnostics of a package that does not check, the C of one
//! that does, a refusal of the C backend, and a build file or a source that
//! cannot be read.

// Stage1 is C, built with a Unix C toolchain; elsewhere only the build file
// is checked.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hex_sha256(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The `.argx` files of a directory of the repository, sorted.
fn argx_in(directory: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(root().join(directory))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("argx"))
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

/// The `(key, value)` lines of a build file, without its header and comments.
fn build_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .skip(1)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (key, value) = line.split_once(' ').unwrap();
            (key.to_string(), value.to_string())
        })
        .collect()
}

/// `argorix.build` describes the same compilation set stage0 uses for
/// `compiler/main.argx` with `--stdlib stdlib`: the root's own directory and
/// the standard library, every module of both.
#[test]
fn the_build_file_names_every_module_of_the_compiler() {
    let text = fs::read_to_string(root().join("argorix.build")).unwrap();
    assert_eq!(text.lines().next(), Some("argorix-build 1"));
    let lines = build_lines(&text);
    let value = |key: &str| -> Vec<String> {
        lines
            .iter()
            .filter(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
            .collect()
    };
    assert_eq!(value("root"), ["compiler/main.argx"]);
    let mut expected: Vec<String> = argx_in("compiler")
        .into_iter()
        .filter(|file| file != "compiler/main.argx")
        .collect();
    expected.extend(argx_in("stdlib"));
    assert_eq!(value("module"), expected);
    assert_eq!(value("c"), ["compiler.c"]);
    assert_eq!(value("manifest"), ["compiler.json"]);
    assert_eq!(value("diagnostics"), ["diagnostics.txt"]);
}

#[cfg(unix)]
fn c_compiler() -> Option<&'static str> {
    ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok())
}

/// A fresh directory for one test.
#[cfg(unix)]
fn work_dir(name: &str) -> PathBuf {
    let work = env::temp_dir().join(format!("argorix-stage1-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    work
}

/// What stage0's `core-emit-c` prints for `file`, run from `directory`, with
/// the standard library when asked: its exit status, the C it wrote, and its
/// error text without the `Error: ` prefix.
#[cfg(unix)]
fn stage0_emit(directory: &Path, file: &str, stdlib: bool, output: &Path) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_argorixc"));
    if stdlib {
        command.arg("--stdlib").arg(root().join("stdlib"));
    }
    let run = command
        .arg("core-emit-c")
        .arg(file)
        .arg("--output")
        .arg(output)
        .current_dir(directory)
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&run.stderr);
    (
        run.status.success(),
        error.strip_prefix("Error: ").unwrap_or(&error).to_string(),
    )
}

/// Compile generated C with the declared profile. The compiler's own trees
/// need more steps and larger buffers than the defaults.
#[cfg(unix)]
fn compile_c(compiler: &str, c_file: &Path, executable: &Path) {
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_STEP_LIMIT=400000000000ULL")
        .arg("-DARGORIX_BUFFER_LIMIT_BYTES=268435456U")
        .arg("-I")
        .arg(root().join("bootstrap/c"))
        .arg(root().join("bootstrap/c/argorix_core_runtime.c"))
        .arg(c_file)
        .arg("-o")
        .arg(executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

/// Stage1, as stage0 builds it: its C and the program.
#[cfg(unix)]
fn build_stage1(compiler: &str, work: &Path) -> (PathBuf, PathBuf) {
    let c_file = work.join("stage1.c");
    let (ok, error) = stage0_emit(&root(), "compiler/main.argx", true, &c_file);
    assert!(ok, "stage0 cannot build stage1: {error}");
    let executable = work.join("stage1");
    compile_c(compiler, &c_file, &executable);
    (c_file, executable)
}

/// Runs a compiler over `package` into a new `build` directory, and returns
/// its result: the number `argorix_main` returned.
#[cfg(unix)]
fn run(compiler: &Path, package: &Path, build: &Path) -> String {
    fs::create_dir_all(build).unwrap();
    let output = Command::new(compiler)
        .arg("--package-root")
        .arg(package)
        .args(["--read-budget", "100000000"])
        .arg("--build-root")
        .arg(build)
        .args(["--write-budget", "100000000"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} failed: {}",
        compiler.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .strip_prefix("ARGORIX_RESULT:")
        .unwrap_or_else(|| panic!("no result from {}", compiler.display()))
        .to_string()
}

/// The files a build wrote, by name.
#[cfg(unix)]
fn written(build: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(build)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Stage1 compiles its own sources to the C stage0 wrote for it, with a
/// manifest that describes them; that C is a compiler that does the same.
#[cfg(unix)]
#[test]
fn stage1_builds_itself_when_cc_is_available() {
    let Some(compiler) = c_compiler() else {
        eprintln!("C compiler unavailable; stage1 is built in C-enabled CI");
        return;
    };
    let work = work_dir("self");
    let (stage0_c, stage1) = build_stage1(compiler, &work);

    let first = work.join("build1");
    assert_eq!(run(&stage1, &root(), &first), "0");
    assert_eq!(
        written(&first),
        ["compiler.c", "compiler.json", "diagnostics.txt"]
    );
    let c = fs::read(first.join("compiler.c")).unwrap();
    assert!(
        c == fs::read(&stage0_c).unwrap(),
        "stage1's C for itself differs from stage0's"
    );
    assert!(fs::read(first.join("diagnostics.txt")).unwrap().is_empty());

    // The manifest names every source with its digest, and the output.
    let text = fs::read_to_string(root().join("argorix.build")).unwrap();
    let files: Vec<String> = build_lines(&text)
        .into_iter()
        .filter(|(key, _)| key == "root" || key == "module")
        .map(|(_, value)| value)
        .collect();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(first.join("compiler.json")).unwrap()).unwrap();
    assert_eq!(manifest["status"], "emitted");
    assert_eq!(manifest["root"], "compiler.main");
    let sources = manifest["sources"].as_array().unwrap();
    assert_eq!(sources.len(), files.len());
    for (source, file) in sources.iter().zip(&files) {
        let bytes = fs::read(root().join(file)).unwrap();
        assert_eq!(source["path"], file.as_str());
        assert_eq!(source["bytes"], bytes.len());
        assert_eq!(source["sha256"], hex_sha256(&bytes));
    }
    assert_eq!(manifest["output"]["bytes"], c.len());
    assert_eq!(manifest["output"]["sha256"], hex_sha256(&c));
    let modules = manifest["modules"].as_array().unwrap();
    assert_eq!(modules.last().unwrap(), "compiler.main");
    assert!(modules.iter().any(|module| module == "compiler.c_emit"));

    // The C stage1 wrote is a compiler: built, it does the same build again.
    let rebuilt = work.join("stage1-rebuilt");
    compile_c(compiler, &first.join("compiler.c"), &rebuilt);
    let second = work.join("build2");
    assert_eq!(run(&rebuilt, &root(), &second), "0");
    for name in ["compiler.c", "compiler.json", "diagnostics.txt"] {
        assert!(
            fs::read(first.join(name)).unwrap() == fs::read(second.join(name)).unwrap(),
            "the rebuilt compiler writes a different {name}"
        );
    }
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}

/// A package in `work/<name>`: `files` under `src/`, and a build file that
/// lists them, the first as the root.
#[cfg(unix)]
fn package(work: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let package = work.join(name);
    fs::create_dir_all(package.join("src")).unwrap();
    let mut build = String::from("argorix-build 1\n");
    for (index, (file, text)) in files.iter().enumerate() {
        fs::write(package.join("src").join(file), text).unwrap();
        let key = if index == 0 { "root" } else { "module" };
        build.push_str(&format!("{key} src/{file}\n"));
    }
    build.push_str("c out.c\nmanifest out.json\ndiagnostics out.txt\n");
    fs::write(package.join("argorix.build"), build).unwrap();
    package
}

const MAIN: &str = "core 0.1;
module demo.main;

import demo.helper;

pub fn argorix_main() -> u32 {
    helper.twice(20u32) + 2u32
}
";

const HELPER: &str = "core 0.1;
module demo.helper;

pub fn twice(value: u32) -> u32 {
    value * 2u32
}
";

/// Each outcome of the driver: what it returns and writes.
#[cfg(unix)]
#[test]
fn stage1_reports_each_outcome_when_cc_is_available() {
    let Some(compiler) = c_compiler() else {
        eprintln!("C compiler unavailable; stage1 is built in C-enabled CI");
        return;
    };
    let work = work_dir("outcomes");
    let (_, stage1) = build_stage1(compiler, &work);

    // A package that checks: the C is stage0's, and it runs.
    let good = package(
        &work,
        "good",
        &[("main.argx", MAIN), ("helper.argx", HELPER)],
    );
    let build = work.join("good-build");
    assert_eq!(run(&stage1, &good, &build), "0");
    assert_eq!(written(&build), ["out.c", "out.json", "out.txt"]);
    let expected = work.join("good-stage0.c");
    let (ok, error) = stage0_emit(&good, "src/main.argx", false, &expected);
    assert!(ok, "{error}");
    assert!(fs::read(build.join("out.c")).unwrap() == fs::read(&expected).unwrap());
    let program = work.join("good-program");
    compile_c(compiler, &build.join("out.c"), &program);
    let output = Command::new(&program).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ARGORIX_RESULT:42\n"
    );

    // A package that does not check, in a dependency and in the root: the
    // diagnostics are stage0's, and there is no C.
    let broken_helper = HELPER.replace("value * 2u32", "value * true");
    let broken_main = MAIN.replace("+ 2u32", "+ missing");
    for (name, main, helper) in [
        ("dependency", MAIN, broken_helper.as_str()),
        ("root", broken_main.as_str(), HELPER),
    ] {
        let failing = package(&work, name, &[("main.argx", main), ("helper.argx", helper)]);
        let build = work.join(format!("{name}-build"));
        assert_eq!(run(&stage1, &failing, &build), "1", "{name}");
        assert_eq!(written(&build), ["out.json", "out.txt"], "{name}");
        let (ok, error) = stage0_emit(&failing, "src/main.argx", false, &work.join("unused.c"));
        assert!(!ok, "{name}");
        assert_eq!(
            fs::read_to_string(build.join("out.txt")).unwrap(),
            error,
            "{name}"
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(build.join("out.json")).unwrap()).unwrap();
        assert_eq!(manifest["status"], "check failed", "{name}");
    }

    // A package the C backend refuses: a result it cannot print.
    let refused = package(
        &work,
        "refused",
        &[(
            "main.argx",
            "core 0.1;\nmodule demo.main;\n\npub fn argorix_main() -> unit {\n    unit\n}\n",
        )],
    );
    let (ok, error) = stage0_emit(&refused, "src/main.argx", false, &work.join("unused.c"));
    assert!(!ok && error.contains("CBackendUnsupported"), "{error}");
    let build = work.join("refused-build");
    assert_eq!(run(&stage1, &refused, &build), "2");
    assert_eq!(written(&build), ["out.json", "out.txt"]);
    assert_eq!(fs::read_to_string(build.join("out.txt")).unwrap(), error);

    // Build files that are not valid: nothing but the reason is written, and
    // only when the `diagnostics` line was read.
    let cases: [(&str, &str, Option<&str>); 6] = [
        ("wrong-header", "argorix-build 2\ndiagnostics out.txt\n", None),
        (
            "unknown-key",
            "argorix-build 1\ndiagnostics out.txt\noutput out.c\n",
            Some("argorix.build:3: unknown key; the keys are `root`, `module`, `c`, `manifest` and `diagnostics`\n"),
        ),
        (
            "repeated-key",
            "argorix-build 1\ndiagnostics out.txt\nroot a.argx\nroot b.argx\n",
            Some("argorix.build:4: this key was already given; only `module` may repeat\n"),
        ),
        (
            "malformed",
            "argorix-build 1\r\ndiagnostics out.txt\r\nroot  a.argx\r\n",
            Some("argorix.build:3: expected `key value`, with one space and no space in the value\n"),
        ),
        (
            "missing-c",
            "argorix-build 1\n# a comment\n\nroot a.argx\nmanifest out.json\ndiagnostics out.txt\n",
            Some("argorix.build: no `c` line\n"),
        ),
        ("empty", "", None),
    ];
    for (name, text, message) in cases {
        let package = work.join(name);
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("argorix.build"), text).unwrap();
        let build = work.join(format!("{name}-build"));
        assert_eq!(run(&stage1, &package, &build), "3", "{name}");
        match message {
            Some(message) => {
                assert_eq!(written(&build), ["out.txt"], "{name}");
                assert_eq!(
                    fs::read_to_string(build.join("out.txt")).unwrap(),
                    message,
                    "{name}"
                );
            }
            None => assert!(written(&build).is_empty(), "{name}"),
        }
    }

    // Files that cannot be read: the host's refusal, as a digest and in words.
    let empty = work.join("no-build-file");
    fs::create_dir_all(&empty).unwrap();
    let build = work.join("no-build-file-build");
    // 1000000 + Host (7) * 1000000 + not found (2).
    assert_eq!(run(&stage1, &empty, &build), "8000002");
    assert!(written(&build).is_empty());
    for (name, path, result, message) in [
        ("missing-module", "src/missing.argx", "8000002", "not found"),
        (
            "escaping-module",
            "../helper.argx",
            "5000000",
            "the path is not a normalized relative path",
        ),
    ] {
        let package = package(&work, name, &[("main.argx", MAIN)]);
        let text = fs::read_to_string(package.join("argorix.build")).unwrap();
        let text = text.replace("c out.c", &format!("module {path}\nc out.c"));
        fs::write(package.join("argorix.build"), text).unwrap();
        let build = work.join(format!("{name}-build"));
        assert_eq!(run(&stage1, &package, &build), result, "{name}");
        assert_eq!(written(&build), ["out.txt"], "{name}");
        assert_eq!(
            fs::read_to_string(build.join("out.txt")).unwrap(),
            format!("{path}: {message}\n"),
            "{name}"
        );
    }
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}
