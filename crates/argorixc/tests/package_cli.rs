use std::{path::PathBuf, process::Command};

fn project(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .args(args)
        .output()
        .expect("argorixc binary runs")
}

#[test]
fn check_package_succeeds() {
    let manifest = project("examples/module_project/argorix.toml");
    let output = run(&["check-package", manifest.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Package entry: app.main"));
    assert!(stdout.contains("Semantic checks: passed"));
}

#[test]
fn check_package_accepts_directory() {
    let dir = project("examples/module_project");
    let output = run(&["check-package", dir.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn emit_ir_package_is_clean_json_with_metadata() {
    let manifest = project("examples/module_project/argorix.toml");
    let output = run(&["emit-ir-package", manifest.to_str().unwrap()]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ir_version"], "0.17");
    assert_eq!(json["module"], "app.main");
    assert_eq!(json["modules"].as_array().unwrap().len(), 6);
    assert_eq!(json["imports"].as_array().unwrap().len(), 5);
}

#[test]
fn emit_bytecode_package_is_clean_json_with_metadata() {
    let manifest = project("examples/module_project/argorix.toml");
    let output = run(&["emit-bytecode-package", manifest.to_str().unwrap()]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["bytecode_version"], "0.17");
    assert_eq!(json["modules"].as_array().unwrap().len(), 6);
}

#[test]
fn graph_package_prints_tree() {
    let manifest = project("examples/module_project/argorix.toml");
    let output = run(&["graph-package", manifest.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("app.main"));
    assert!(stdout.contains("└── tools.search"));
}

#[test]
fn unknown_import_package_fails() {
    let manifest = project("examples/invalid_modules/unknown_import/argorix.toml");
    let output = run(&["check-package", manifest.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown module"));
}

#[test]
fn single_file_check_still_works() {
    let file = project("examples/provider_allowlists_v016.argx");
    let output = run(&["check", file.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Argorix Lang compiler v0.17"));
}
