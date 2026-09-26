//! ESP-018.A: the Argorix agent-language lexer (`compiler/agent_lexer.argx`)
//! against the stage0 one (`argorix_parser::lexer`).
//!
//! `tests/selfhost/agent/lex_files.argx` is compiled through the transitional
//! C backend and run with a package root holding every sample. It reads each
//! sample through the compiler-host boundary, lexes it with the Argorix lexer
//! and writes its canonical token dump (`spec/language/tokens.md`). Each dump
//! must equal, byte for byte, what the stage0 lexer produces (`argorixc
//! agent-tokens`). The samples are the adversarial cases in
//! `tests/selfhost/agent/samples/` and every `.argx` file of the repository:
//! the agent-language corpus, and Core sources, which the agent lexer must
//! tokenize the same way too.

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_parser::lexer::token_dump;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `directory` with the extension, as sorted repository
/// paths, skipping build output and version control.
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

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut names = Vec::new();
    walk(
        &root().join("tests/selfhost/agent/samples"),
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
        .collect()
}

#[test]
fn the_samples_cover_every_kind_of_token_and_error() {
    let dumps: String = samples()
        .iter()
        .filter(|(name, _)| name.starts_with("tests/selfhost/agent/samples/"))
        .map(|(_, source)| token_dump(source))
        .collect();
    for needle in [
        " Ident ",
        " String ",
        " Integer ",
        " LeftBrace",
        " RightBrace",
        " LeftParen",
        " RightParen",
        " LeftBracket",
        " RightBracket",
        " Comma",
        " Colon",
        " Arrow",
        " Eof",
        "unexpected character",
        "unterminated string literal",
        "integer literal exceeds u64 range",
        "source is not valid UTF-8",
    ] {
        assert!(dumps.contains(needle), "no sample gives `{needle}`");
    }
}

#[cfg(unix)]
#[test]
fn the_argorix_agent_lexer_matches_the_stage0_lexer_when_cc_is_available() {
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

    let work = env::temp_dir().join(format!("argorix-agent-lexer-{}", std::process::id()));
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
        .arg(root().join("tests/selfhost/agent/lex_files.argx"))
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
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        // Lexing the whole corpus takes more than the default budget of steps.
        .arg("-DARGORIX_STEP_LIMIT=40000000000ULL")
        // The token buffers of the largest sources pass the 1 MiB default.
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

    let mut failed = Vec::new();
    for (index, (name, source)) in samples.iter().enumerate() {
        let expected = token_dump(source);
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
        "the Argorix agent lexer disagrees on {} of {} samples:\n{}",
        failed.len(),
        samples.len(),
        failed.join("\n")
    );
    fs::remove_dir_all(work).unwrap();
}
