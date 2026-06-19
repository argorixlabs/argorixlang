use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("argorix-v014-cli-{}-{name}", std::process::id()))
}

fn copy_fixture(root: &std::path::Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(format!("examples/{name}.argbc.json"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::copy(source, &path).unwrap();
    path
}

#[test]
fn run_exports_portable_bundle_trace_and_report_without_json_noise() {
    let root = temp_root("success");
    let bytecode = copy_fixture(
        &root,
        "program",
        "../../examples/provider_allowlists_v013.argbc.json",
    );
    let report = root.join("reports/run.security.json");
    let trace = root.join("reports/run.trace.json");
    let bundle = root.join("reports/run.bundle.json");
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("run")
        .arg(&bytecode)
        .args([
            "--dry-run",
            "--reactive",
            "--inject",
            "User:ResearchAgent:tell:UserPrompt",
            "--json",
            "--security-report",
        ])
        .arg(&report)
        .arg("--trace-out")
        .arg(&trace)
        .arg("--evidence-bundle")
        .arg(&bundle)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["mode"], "reactive-dry-run");
    assert!(report.exists());
    assert!(trace.exists());
    let bundle_json: Value = serde_json::from_slice(&fs::read(&bundle).unwrap()).unwrap();
    assert_eq!(bundle_json["bundle_version"], "0.17");
    assert_eq!(
        bundle_json["artifacts"]["trace_path"],
        Value::String("run.trace.json".into())
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("written"));

    let verify = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("verify-evidence")
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&verify.stdout).trim(),
        "Evidence verification: passed"
    );

    let verify_json = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("verify-evidence")
        .arg(&bundle)
        .arg("--json")
        .output()
        .unwrap();
    let result: Value = serde_json::from_slice(&verify_json.stdout).unwrap();
    assert_eq!(result["passed"], true);
    assert_eq!(result["checks_failed"], 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_run_still_exports_report_and_bundle_without_trace() {
    let root = temp_root("failed");
    let bytecode = copy_fixture(
        &root,
        "invalid",
        "../../examples/invalid_bytecode_missing_end.argbc.json",
    );
    let report = root.join("reports/run.security.json");
    let trace = root.join("reports/run.trace.json");
    let bundle = root.join("reports/run.bundle.json");
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("run")
        .arg(&bytecode)
        .args([
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Receiver:tell:Message",
            "--json",
            "--security-report",
        ])
        .arg(&report)
        .arg("--trace-out")
        .arg(&trace)
        .arg("--evidence-bundle")
        .arg(&bundle)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(report.exists());
    assert!(bundle.exists());
    assert!(!trace.exists());
    let bundle_json: Value = serde_json::from_slice(&fs::read(&bundle).unwrap()).unwrap();
    assert!(bundle_json["trace_digest"].is_null());
    assert!(bundle_json["artifacts"]["trace_path"].is_null());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verify_evidence_exits_nonzero_and_reports_digest_mismatch() {
    let root = temp_root("tamper");
    let bytecode = copy_fixture(
        &root,
        "program",
        "../../examples/provider_allowlists_v013.argbc.json",
    );
    let report = root.join("reports/run.security.json");
    let trace = root.join("reports/run.trace.json");
    let bundle = root.join("reports/run.bundle.json");
    let run = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("run")
        .arg(&bytecode)
        .args([
            "--dry-run",
            "--reactive",
            "--inject",
            "User:ResearchAgent:tell:UserPrompt",
            "--security-report",
        ])
        .arg(&report)
        .arg("--trace-out")
        .arg(&trace)
        .arg("--evidence-bundle")
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(run.status.success());

    let mut report_json: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    report_json["module"] = Value::String("Tampered".into());
    fs::write(
        &report,
        format!("{}\n", serde_json::to_string_pretty(&report_json).unwrap()),
    )
    .unwrap();

    let verify = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("verify-evidence")
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(!verify.status.success());
    assert!(String::from_utf8_lossy(&verify.stdout).contains("- report_digest mismatch"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verify_evidence_resolves_relative_bundle_path_from_its_parent() {
    let root = temp_root("relative-bundle");
    let bytecode = copy_fixture(
        &root,
        "program",
        "../../examples/provider_allowlists_v014.argbc.json",
    );
    let report = root.join("reports/run.security.json");
    let trace = root.join("reports/run.trace.json");
    let bundle = root.join("reports/run.bundle.json");
    let run = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .arg("run")
        .arg(&bytecode)
        .args([
            "--dry-run",
            "--reactive",
            "--inject",
            "User:ResearchAgent:tell:UserPrompt",
            "--security-report",
        ])
        .arg(&report)
        .arg("--trace-out")
        .arg(&trace)
        .arg("--evidence-bundle")
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(run.status.success());

    let verify = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .current_dir(&root)
        .args(["verify-evidence", "reports/run.bundle.json"])
        .output()
        .unwrap();

    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    fs::remove_dir_all(root).unwrap();
}
