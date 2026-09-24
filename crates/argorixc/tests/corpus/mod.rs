//! The package corpus the differentials share: every package is a root and
//! its locked set (`link_differential.rs`, `native.rs`).

use std::fs;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The files of a directory with one of `extensions`, sorted, as paths
/// relative to the repository.
pub fn files(directory: &str, extensions: &[&str]) -> Vec<String> {
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
pub fn packages() -> Vec<(String, Vec<String>)> {
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
        "tests/selfhost/stage1",
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
        "tests/selfhost/stage1/refusals",
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
