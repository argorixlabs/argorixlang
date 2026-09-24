//! ESP-012.C and ESP-013.A/C: the Argorix linker and checker
//! (`compiler/link.argx`), IR lowering (`compiler/ir.argx`) and C backend
//! (`compiler/c_emit.argx`) against stage0.
//!
//! A harness is compiled through the transitional C backend and run with a
//! package root holding every source file. It reads each package through the
//! compiler-host boundary and writes one dump per package, which must equal,
//! byte for byte, what stage0 produces for the same files:
//!
//! - `tests/selfhost/check/link_files.argx` writes the diagnostics dump
//!   (`spec/core/check.md`), against the stage0 driver's `check_core_package`
//!   (`argorixc core-check-package-dump`);
//! - `tests/selfhost/ir/ir_files.argx` writes the canonical IR JSON
//!   (`spec/core/ir.md`), against `argorix_ir::core_package_ir_dump`;
//! - `tests/selfhost/c/c_files.argx` writes the transitional C
//!   (`spec/core/c-backend.md`), against `argorix_ir::core_package_c_dump`.
//!
//! The packages are the compiler, which links the Argorix checker and linker
//! with themselves, each module of the standard library, the programs that
//! import them, the multi-module conformance programs, the single files of the
//! other corpora and the cases in `tests/selfhost/check/packages/`.

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_ir::{core_package_c_dump, core_package_ir_dump};
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
fn on_large_stack(oracle: fn(&[Vec<u8>]) -> String, sources: Vec<Vec<u8>>) -> String {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || oracle(&sources))
        .unwrap()
        .join()
        .unwrap()
}

/// The `.argx` files of a directory, sorted, relative to the repository.
fn files_in(directory: &str) -> Vec<String> {
    files(directory, &["argx"])
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
fn packages_list() -> Vec<(String, Vec<String>)> {
    packages()
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
    differential(
        "tests/selfhost/check/link_files.argx",
        "link",
        core_package_check_dump,
    );
}

#[cfg(unix)]
#[test]
fn the_argorix_ir_matches_the_stage0_ir_when_cc_is_available() {
    differential(
        "tests/selfhost/ir/ir_files.argx",
        "ir",
        core_package_ir_dump,
    );
}

#[cfg(unix)]
#[test]
fn the_argorix_c_backend_matches_the_stage0_backend_when_cc_is_available() {
    differential("tests/selfhost/c/c_files.argx", "c", core_package_c_dump);
}

#[cfg(unix)]
fn differential(harness: &str, extension: &str, oracle: fn(&[Vec<u8>]) -> String) {
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

    let work = env::temp_dir().join(format!("argorix-{extension}-{}", std::process::id()));
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

    let c_file = work.join("harness.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join(harness))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("harness");
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
        let expected = on_large_stack(oracle, sources);
        let actual = fs::read(build.join(format!("dumps/{index}.{extension}"))).unwrap();
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
        "the Argorix {extension} dump disagrees:\n{}",
        failed.join("\n")
    );
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}

/// Compile generated C with the declared profile and the budgets the
/// compiler's own trees need.
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

/// The C backend written in Argorix compiles itself, and what it compiles
/// compiles itself again to the same bytes. Stage0 builds
/// `tests/selfhost/c/c_files.argx` (the lexer, parser, linker, checker, IR
/// lowering and C backend of `compiler/`); that program emits C for its own
/// package; the C is compiled, and the new program emits C for the same
/// package once more. All three C files must be identical.
#[cfg(unix)]
#[test]
fn the_argorix_c_backend_reproduces_itself_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the fixed point runs in C-enabled CI");
        return;
    };
    let work = env::temp_dir().join(format!("argorix-fixed-point-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package_root = work.join("package");
    fs::create_dir_all(package_root.join("sources")).unwrap();
    let mut files = vec!["tests/selfhost/c/c_files.argx".to_string()];
    files.extend(files_in("compiler"));
    files.extend(files_in("stdlib"));
    let mut list = Vec::new();
    let mut total = 0_u64;
    for (index, file) in files.iter().enumerate() {
        let source = fs::read(root().join(file)).unwrap();
        let copy = format!("sources/{index}.src");
        fs::write(package_root.join(&copy), &source).unwrap();
        total += source.len() as u64;
        list.push(copy);
    }
    let list = format!("{}\n", list.join(" "));
    fs::write(package_root.join("packages.txt"), &list).unwrap();
    total += list.len() as u64;

    let stage0_c = work.join("stage0.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/c/c_files.argx"))
        .arg("--output")
        .arg(&stage0_c)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let mut previous = stage0_c;
    for generation in 1..=2 {
        let executable = work.join(format!("c_files_{generation}"));
        compile_c(compiler, &previous, &executable);
        let build = work.join(format!("build_{generation}"));
        fs::create_dir_all(build.join("dumps")).unwrap();
        let run = Command::new(&executable)
            .arg("--package-root")
            .arg(&package_root)
            .arg("--read-budget")
            .arg(total.to_string())
            .arg("--build-root")
            .arg(&build)
            .args(["--write-budget", "100000000"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&run.stdout).trim(),
            "ARGORIX_RESULT:1",
            "generation {generation}, stderr: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        let emitted = build.join("dumps/0.c");
        assert!(
            fs::read(&emitted).unwrap() == fs::read(&previous).unwrap(),
            "generation {generation} emitted different C"
        );
        previous = emitted;
    }
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}

/// ESP-013.D: the pipeline (`compiler/pipeline.argx`) builds every package to
/// the C stage0 writes, and describes each build in a manifest whose every
/// field is recomputed here without Argorix: SHA-256 with the `sha2` crate,
/// the link order with stage0. Files of every length from 0 to 130 bytes,
/// which do not parse, are built too, to cover every padding case of the
/// digest.
#[cfg(unix)]
#[test]
fn the_argorix_pipeline_builds_and_describes_every_package_when_cc_is_available() {
    // Parsing the deep-nesting samples in stage0 needs the compiler's stack.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(pipeline_builds_and_describes_every_package)
        .unwrap()
        .join()
        .unwrap();
}

#[cfg(unix)]
fn pipeline_builds_and_describes_every_package() {
    use argorix_semantics::{core_link_order, core_package};
    use sha2::{Digest, Sha256};
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the pipeline test runs in C-enabled CI");
        return;
    };
    let work = env::temp_dir().join(format!("argorix-pipeline-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package_root = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package_root.join("samples")).unwrap();
    fs::create_dir_all(build.join("out")).unwrap();
    // Each package as the files' copied names and contents, root first.
    let mut packages: Vec<Vec<(String, Vec<u8>)>> = Vec::new();
    let mut copies: BTreeMap<String, String> = BTreeMap::new();
    for (_, files) in packages_list() {
        let mut package = Vec::new();
        for file in files {
            let next = copies.len();
            let copy = copies
                .entry(file.clone())
                .or_insert_with(|| format!("samples/{next}.src"))
                .clone();
            package.push((copy, fs::read(root().join(&file)).unwrap()));
        }
        packages.push(package);
    }
    for length in 0..=130_usize {
        let contents: Vec<u8> = (0..length).map(|at| (at * 7 + length) as u8).collect();
        packages.push(vec![(format!("samples/length_{length}.bin"), contents)]);
    }
    let mut list = String::new();
    let mut total = 0_u64;
    for package in &packages {
        for (path, contents) in package {
            fs::write(package_root.join(path), contents).unwrap();
            total += contents.len() as u64;
        }
        let names: Vec<&str> = package.iter().map(|(path, _)| path.as_str()).collect();
        list.push_str(&names.join(" "));
        list.push('\n');
    }
    fs::write(package_root.join("packages.txt"), &list).unwrap();

    let c_file = work.join("build_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/pipeline/build_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("build_files");
    compile_c(compiler, &c_file, &executable);
    let run = Command::new(&executable)
        .arg("--package-root")
        .arg(&package_root)
        .arg("--read-budget")
        .arg((total + list.len() as u64).to_string())
        .arg("--build-root")
        .arg(&build)
        .args(["--write-budget", "200000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", packages.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let hex = |data: &[u8]| format!("{:x}", Sha256::digest(data));
    for (index, package) in packages.iter().enumerate() {
        let sources: Vec<Vec<u8>> = package.iter().map(|(_, data)| data.clone()).collect();
        let c = fs::read(build.join(format!("out/{index}.c"))).unwrap();
        let expected_c = on_large_stack(core_package_c_dump, sources.clone());
        assert!(
            c == expected_c.as_bytes(),
            "package {index}: the pipeline's C differs from stage0's"
        );
        let text = fs::read_to_string(build.join(format!("out/{index}.json"))).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let status = match expected_c.as_str() {
            "check failed\n" => "check failed",
            "unsupported\n" => "unsupported",
            _ => "emitted",
        };
        assert_eq!(manifest["status"], status, "package {index}");
        assert_eq!(manifest["output"]["bytes"], c.len(), "package {index}");
        assert_eq!(manifest["output"]["sha256"], hex(&c), "package {index}");
        let recorded = manifest["sources"].as_array().unwrap();
        assert_eq!(recorded.len(), package.len(), "package {index}");
        for ((path, data), entry) in package.iter().zip(recorded) {
            assert_eq!(entry["path"], path.as_str(), "package {index}");
            assert_eq!(entry["bytes"], data.len(), "package {index}");
            assert_eq!(entry["sha256"], hex(data), "package {index}: {path}");
            let module = std::str::from_utf8(data)
                .ok()
                .and_then(|text| argorix_parser::core::parse_core_source(text).ok())
                .map(|program| program.module.value);
            match module {
                Some(name) => assert_eq!(entry["module"], name.as_str(), "package {index}"),
                None => assert!(entry["module"].is_null(), "package {index}"),
            }
        }
        let order: Vec<String> = match core_package(&sources) {
            Ok(loaded) => core_link_order(&loaded.root, &loaded.modules, &loaded.duplicates)
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let modules: Vec<String> = manifest["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_string())
            .collect();
        assert_eq!(modules, order, "package {index}");
    }
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}

/// ESP-013.B: the Argorix IR verifier (`compiler/ir_verify.argx`) against
/// `argorix_ir::core_ir_verify_dump`. The documents are the IR of every
/// package of the differential that lowers, compact and pretty-printed, and
/// mutations of one of them that each break one rule of the verifier, or the
/// document's shape.
#[cfg(unix)]
#[test]
fn the_argorix_ir_verifier_matches_stage0_when_cc_is_available() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(verifier_matches_stage0)
        .unwrap()
        .join()
        .unwrap();
}

#[cfg(unix)]
fn verifier_mutations(base: &serde_json::Value) -> Vec<(String, Vec<u8>)> {
    use serde_json::{json, Value};
    let mut found: Vec<(String, Vec<u8>)> = Vec::new();
    let mut add = |name: &str, value: Value| {
        found.push((name.into(), serde_json::to_vec(&value).unwrap()));
    };
    let mutate = |change: &dyn Fn(&mut Value)| {
        let mut value = base.clone();
        change(&mut value);
        value
    };
    add("ir_version", mutate(&|v| v["ir_version"] = json!("0.2")));
    add(
        "core_version",
        mutate(&|v| v["core_version"] = json!("0.9")),
    );
    add(
        "duplicate_effect",
        mutate(&|v| {
            v["effect_policy"] = json!([{"effect": "trap"}, {"effect": "trap"}]);
        }),
    );
    add(
        "unauthorized_effect",
        mutate(&|v| {
            v["effect_policy"] = json!([
                {"effect": "host", "detail": "network.connect"},
                {"effect": "host", "detail": "clock.read"},
                {"effect": "host", "detail": "network.connect"}
            ]);
        }),
    );
    add(
        "undeclared_effect",
        mutate(&|v| v["effect_policy"] = json!([])),
    );
    add(
        "duplicate_lock",
        mutate(&|v| v["locked_modules"] = json!(["demo.main", "demo.main"])),
    );
    add(
        "import_not_locked",
        mutate(&|v| {
            v["imports"] =
                json!([{"path": "demo.elsewhere"}, {"path": "demo.main", "alias": "self_"}])
        }),
    );
    add("blank_module", mutate(&|v| v["module"] = json!("  \t")));
    add(
        "unknown_name",
        mutate(&|v| {
            let items = v["items"].as_array_mut().unwrap();
            let last = items.last_mut().unwrap();
            last["body"]["tail"] = json!({"expression": "path", "segments": ["nope"]});
        }),
    );
    add(
        "return_type",
        mutate(&|v| {
            let items = v["items"].as_array_mut().unwrap();
            items.last_mut().unwrap()["return_type"] = json!({"type": "named", "name": "bool"});
        }),
    );
    add(
        "continue_outside_loop",
        mutate(&|v| {
            let items = v["items"].as_array_mut().unwrap();
            let body = &mut items.last_mut().unwrap()["body"]["statements"];
            body.as_array_mut()
                .unwrap()
                .insert(0, json!({"statement": "continue"}));
        }),
    );
    add(
        "host_parameter",
        mutate(&|v| {
            let items = v["items"].as_array_mut().unwrap();
            items.last_mut().unwrap()["parameters"] =
                json!([{"name": "package", "ty": {"type": "named", "name": "PackageRead"}}]);
        }),
    );
    add(
        "items_missing",
        mutate(&|v| {
            v.as_object_mut().unwrap().remove("items");
        }),
    );
    add("imports_null", mutate(&|v| v["imports"] = Value::Null));
    add(
        "imports_absent",
        mutate(&|v| {
            v.as_object_mut().unwrap().remove("imports");
        }),
    );
    add(
        "unknown_expression",
        mutate(&|v| {
            let items = v["items"].as_array_mut().unwrap();
            items.last_mut().unwrap()["body"]["tail"] = json!({"expression": "lambda"});
        }),
    );
    add(
        "extra_fields",
        mutate(&|v| {
            v["comment"] = json!({"nested": [1, 2.5, -3, "x\u{1f600}"]});
        }),
    );
    for depth in [122_usize, 123, 124] {
        let mut expression = json!({"expression": "bool", "value": true});
        for _ in 0..depth {
            expression = json!({"expression": "unary", "operator": "not", "value": expression});
        }
        add(
            &format!("depth_{depth}"),
            json!({
                "ir_version": "0.1", "core_version": "0.1", "module": "m",
                "items": [{"public": true, "item": "const", "name": "C",
                           "ty": {"type": "named", "name": "bool"}, "value": expression}]
            }),
        );
    }
    for (name, text) in [
        ("truncated", "{\"ir_version\": \"0.1\""),
        ("trailing", "{\"ir_version\":\"0.1\",\"core_version\":\"0.1\",\"module\":\"m\",\"items\":[]} x"),
        ("not_an_object", "[1, 2, 3]"),
        ("escapes", "{\"ir_version\":\"0\\u002e1\",\"core_version\":\"0.1\",\"module\":\"a\\/b\",\"items\":[]}"),
        ("empty_program", " {\n \"items\" : [ ] , \"module\":\"m\",\"core_version\":\"0.1\",\"ir_version\":\"0.1\" }\n"),
    ] {
        found.push((name.into(), text.as_bytes().to_vec()));
    }
    found
}

#[cfg(unix)]
fn verifier_matches_stage0() {
    use argorix_ir::core_ir_verify_dump;
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the verifier differential runs in C-enabled CI");
        return;
    };
    let mut documents: Vec<(String, Vec<u8>)> = Vec::new();
    for (root_file, package) in packages() {
        let sources: Vec<Vec<u8>> = package
            .iter()
            .map(|file| fs::read(root().join(file)).unwrap())
            .collect();
        let ir = core_package_ir_dump(&sources);
        if ir == "check failed\n" {
            continue;
        }
        // IR nested past serde_json's recursion limit cannot be read back,
        // by stage0 or by the Argorix verifier; it stays in the corpus.
        let value: Option<serde_json::Value> = serde_json::from_str(&ir).ok();
        documents.push((root_file.clone(), ir.into_bytes()));
        if let Some(value) = value {
            documents.push((
                format!("{root_file} (pretty)"),
                serde_json::to_vec_pretty(&value).unwrap(),
            ));
        }
    }
    let base_sources: Vec<Vec<u8>> = ["main.argx", "helper.argx", "helper_two.argx"]
        .iter()
        .map(|file| {
            fs::read(
                root()
                    .join("tests/selfhost/check/packages/values")
                    .join(file),
            )
            .unwrap()
        })
        .collect();
    let base: serde_json::Value =
        serde_json::from_str(&core_package_ir_dump(&base_sources)).unwrap();
    documents.extend(verifier_mutations(&base));

    let work = env::temp_dir().join(format!("argorix-verify-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package_root = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package_root.join("documents")).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
    let mut list = String::new();
    let mut total = 0_u64;
    for (index, (_, document)) in documents.iter().enumerate() {
        let file = format!("documents/{index}.json");
        fs::write(package_root.join(&file), document).unwrap();
        list.push_str(&file);
        list.push('\n');
        total += document.len() as u64;
    }
    fs::write(package_root.join("files.txt"), &list).unwrap();
    let c_file = work.join("verify_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/ir/verify_files.argx"))
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
    compile_c(compiler, &c_file, &executable);
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
        format!("ARGORIX_RESULT:{}", documents.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut failed = Vec::new();
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    for (index, (name, document)) in documents.iter().enumerate() {
        let expected = core_ir_verify_dump(document);
        *outcomes.entry(expected.clone()).or_default() += 1;
        let actual = fs::read_to_string(build.join(format!("dumps/{index}.verify"))).unwrap();
        if actual != expected {
            failed.push(format!("{name}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(
        failed.is_empty(),
        "the Argorix IR verifier disagrees:\n{}",
        failed.join("\n")
    );
    eprintln!(
        "verifier outcomes over {} documents: {outcomes:?}",
        documents.len()
    );
    // Every rule of the verifier is exercised.
    for code in [
        "IrVersionUnsupported",
        "CoreVersionUnsupported",
        "DuplicateEffect",
        "UnauthorizedEffect",
        "UndeclaredEffect",
        "DuplicateLockedModule",
        "ImportNotLocked",
        "InvalidModule",
        "ImmutableAssignmentOrUnknownName",
        "TypeMismatch",
        "ControlOutsideLoop",
        "decode failed",
        "ok",
    ] {
        assert!(
            outcomes.keys().any(|outcome| outcome.contains(code)),
            "no document exercises {code}"
        );
    }
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}
