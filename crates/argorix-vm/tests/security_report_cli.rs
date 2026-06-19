use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};

fn temp_report(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("argorix-v013-{}-{name}", std::process::id()))
        .join("nested")
        .join("run.security.json")
}

#[test]
fn json_stdout_remains_trace_only_and_parent_directory_is_created() {
    let report_path = temp_report("success");
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .args([
            "run",
            "../../examples/provider_allowlists_v012.argbc.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:ResearchAgent:tell:UserPrompt",
            "--json",
            "--security-report",
        ])
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["mode"], "reactive-dry-run");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Security report written"));
    let report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(report["report_version"], "0.18");
    assert_eq!(report["execution"]["failed"], false);

    fs::remove_dir_all(report_path.ancestors().nth(2).unwrap()).unwrap();
}

#[test]
fn failed_real_cli_writes_report_without_partial_json_and_exits_nonzero() {
    let report_path = temp_report("failure");
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-vm"))
        .args([
            "run",
            "../../examples/invalid_bytecode_missing_end.argbc.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Receiver:tell:Message",
            "--json",
            "--security-report",
        ])
        .arg(&report_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("bytecode verification failed"));
    let report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(report["execution"]["failed"], true);
    assert_eq!(report["verdict"]["severity"], "high");
    assert_eq!(report["ledger"]["last_event"], "VmFailed");

    fs::remove_dir_all(report_path.ancestors().nth(2).unwrap()).unwrap();
}
