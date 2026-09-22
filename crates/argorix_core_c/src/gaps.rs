//! Known gaps of the transitional C backend (issue #27).
//!
//! Each program is valid Core 0.1 that `argorixc core-check` accepts, recorded
//! with the defect it reproduces today. A gap that starts passing is reported
//! so it can be promoted into `tests/selfhost/runtime/cases.json`; a gap that
//! fails in a new way means the record is stale and fails the run.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::harness::{self, Execution, Host, ToolRecord, Toolchain};
use crate::policy::Policy;

pub const STILL_OPEN: &str = "STILL_OPEN";
pub const FIXED: &str = "FIXED";
pub const CHANGED: &str = "CHANGED";
pub const SKIPPED: &str = "SKIPPED";

#[derive(Debug, Clone, Deserialize)]
pub struct GapManifest {
    pub schema_version: u32,
    pub recorded_commit: Option<String>,
    pub gaps: Vec<Gap>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Gap {
    pub id: String,
    pub file: String,
    pub kind: String,
    pub spec_expected_stdout: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub observed_stdout: Option<String>,
    #[serde(default)]
    pub observed_exit: Option<i32>,
    #[serde(default)]
    pub note: String,
    /// Compilers this gap is recorded for, empty meaning every one of them.
    ///
    /// A defect of the declared `-Werror` profile can belong to one compiler
    /// alone: GCC does not implement `-Wself-assign`, so g23 is invisible to
    /// it. Checking such a gap with the wrong compiler would report it fixed
    /// on every run, which is why it is skipped instead.
    #[serde(default)]
    pub compilers: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Outcome {
    pub emit_exit: i32,
    pub emit_stderr: String,
    pub compile_exit: Option<i32>,
    pub compile_diagnostics: String,
    pub execution: Option<Execution>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GapResult {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub detail: String,
    pub observed: Outcome,
}

#[derive(Debug, Clone, Serialize)]
pub struct GapReport {
    pub schema_version: u32,
    pub task: String,
    pub scope: String,
    pub commit: Option<String>,
    pub recorded_commit: Option<String>,
    pub host: Host,
    pub compiler: ToolRecord,
    pub argorixc: ToolRecord,
    pub gaps_total: usize,
    pub still_open: usize,
    pub skipped: usize,
    pub fixed: usize,
    pub changed: usize,
    pub gaps: Vec<GapResult>,
    pub overall_pass: bool,
}

/// Whether this gap is recorded for the compiler in hand.
///
/// The compiler is matched by name inside the resolved executable, so `clang`
/// covers `clang-14` and `/usr/bin/clang` alike.
pub fn applies_to(gap: &Gap, compiler_id: &str) -> bool {
    gap.compilers.is_empty()
        || gap
            .compilers
            .iter()
            .any(|name| compiler_id.contains(name.as_str()))
}

/// Compare one gap's observed behaviour with what the manifest recorded.
pub fn classify(gap: &Gap, outcome: &Outcome) -> (String, String) {
    let still = |detail: String| (STILL_OPEN.to_string(), detail);
    let fixed = |detail: String| (FIXED.to_string(), detail);
    let changed = |detail: String| (CHANGED.to_string(), detail);

    if gap.kind == "emit_rejected" && outcome.emit_exit != 0 {
        return if outcome.emit_stderr.contains(&gap.signature) {
            still(format!("emission still rejects it: {}", gap.signature))
        } else {
            changed(format!(
                "emission fails with a different message: {}",
                outcome.emit_stderr
            ))
        };
    }
    if outcome.emit_exit != 0 {
        return changed(format!("emission now fails: {}", outcome.emit_stderr));
    }
    let Some(compile_exit) = outcome.compile_exit else {
        return changed("emission succeeded but nothing was compiled".into());
    };
    if compile_exit != 0 {
        return if gap.kind == "compile_error"
            && outcome.compile_diagnostics.contains(&gap.signature)
        {
            still(format!("C compilation still fails: {}", gap.signature))
        } else {
            changed(format!(
                "C compilation fails: {}",
                first_error(&outcome.compile_diagnostics)
            ))
        };
    }
    let Some(execution) = outcome.execution.as_ref() else {
        return changed("compiled but never executed".into());
    };
    let stdout = harness::strip_one_newline(&execution.stdout);
    let matches_spec = stdout == gap.spec_expected_stdout && execution.exit == Some(0);
    if gap.kind == "crash" {
        if matches_spec {
            return fixed("the program now runs to the expected result".into());
        }
        if execution.exit == Some(70) && execution.stderr.starts_with("ARGORIX_TRAP:") {
            return fixed(format!(
                "now a typed trap: {}",
                harness::strip_one_newline(&execution.stderr)
            ));
        }
        if execution.exit == gap.observed_exit {
            return still(format!(
                "still exits {:?} instead of a typed trap",
                execution.exit
            ));
        }
        return changed(format!("exit {:?}, stdout {stdout:?}", execution.exit));
    }
    if matches_spec {
        return fixed("the program now produces the result the spec requires".into());
    }
    if gap.kind == "wrong_result"
        && Some(stdout.to_string()) == gap.observed_stdout
        && execution.exit == gap.observed_exit
    {
        return still(format!(
            "still prints {stdout:?} instead of {:?}",
            gap.spec_expected_stdout
        ));
    }
    changed(format!(
        "exit {:?}, stdout {stdout:?}, stderr {:?}",
        execution.exit,
        harness::strip_one_newline(&execution.stderr)
    ))
}

fn first_error(diagnostics: &str) -> String {
    diagnostics
        .lines()
        .find(|line| line.contains("error:"))
        .unwrap_or("")
        .trim()
        .to_string()
}

fn observe(
    gap: &Gap,
    argorixc: &Path,
    root: &Path,
    work: &Path,
    compiler: &Path,
    toolchain: &Toolchain,
    policy: &Policy,
) -> Result<Outcome> {
    let source = root.join("conformance/core_c/gaps").join(&gap.file);
    let generated = work.join(format!("{}.c", gap.id));
    let (emit_exit, emit_stderr) = harness::emit_once(argorixc, root, &source, &generated)?;
    let mut outcome = Outcome {
        emit_exit,
        emit_stderr,
        ..Outcome::default()
    };
    if emit_exit != 0 || !generated.is_file() {
        return Ok(outcome);
    }
    let executable = work.join(&gap.id);
    let argv = harness::compile_argv(compiler, toolchain, root, &[generated], &executable)?;
    let (code, _, diagnostics) = harness::run_argv(&argv)?;
    outcome.compile_exit = Some(code);
    outcome.compile_diagnostics = diagnostics.trim().to_string();
    if code != 0 || !executable.is_file() {
        return Ok(outcome);
    }
    outcome.execution = Some(harness::execute(
        &executable,
        Duration::from_secs(policy.execution_timeout_seconds),
    )?);
    Ok(outcome)
}

pub fn check(argorixc: &Path, root: &Path, compiler_name: &str, work: &Path) -> Result<GapReport> {
    let manifest_path = root.join("conformance/core_c/gaps/gaps.json");
    let manifest: GapManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )?;
    anyhow::ensure!(
        manifest.schema_version == 1,
        "unsupported gaps schema_version"
    );
    anyhow::ensure!(
        argorixc.is_file(),
        "argorixc not found: {}",
        argorixc.display()
    );
    anyhow::ensure!(
        harness::emitter_available(argorixc)?,
        "argorixc has no `core-emit-c` command"
    );
    let toolchain: Toolchain = serde_json::from_str(&std::fs::read_to_string(
        root.join("bootstrap/c/toolchain.json"),
    )?)?;
    let policy = Policy::load(&root.join("conformance/core_c/policy.json"))?;
    let compiler = harness::resolve_compiler(compiler_name, &toolchain)?;
    std::fs::create_dir_all(work)?;

    let compiler_id = compiler
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut results = Vec::new();
    for gap in &manifest.gaps {
        if !applies_to(gap, &compiler_id) {
            results.push(GapResult {
                id: gap.id.clone(),
                kind: gap.kind.clone(),
                status: SKIPPED.to_string(),
                detail: format!("recorded for {} only", gap.compilers.join(", ")),
                observed: Outcome::default(),
            });
            continue;
        }
        let outcome = observe(gap, argorixc, root, work, &compiler, &toolchain, &policy)?;
        let (status, detail) = classify(gap, &outcome);
        results.push(GapResult {
            id: gap.id.clone(),
            kind: gap.kind.clone(),
            status,
            detail,
            observed: outcome,
        });
    }
    let count = |status: &str| results.iter().filter(|item| item.status == status).count();
    let changed = count(CHANGED);
    Ok(GapReport {
        schema_version: 1,
        task: "ESP-008.R gap corpus".into(),
        scope: "Known defects of the transitional C backend (issue #27). A gap that starts \
passing must be promoted into tests/selfhost/runtime/cases.json."
            .into(),
        commit: harness::git_commit(root),
        recorded_commit: manifest.recorded_commit.clone(),
        host: harness::host(),
        compiler: ToolRecord {
            path: compiler.display().to_string(),
            sha256: String::new(),
            version: harness::first_line(&compiler, &["--version"]),
        },
        argorixc: ToolRecord {
            path: harness::relative(argorixc, root),
            sha256: harness::sha256_file(argorixc)?,
            version: harness::first_line(argorixc, &["--version"]),
        },
        gaps_total: results.len(),
        still_open: count(STILL_OPEN),
        skipped: count(SKIPPED),
        fixed: count(FIXED),
        changed,
        gaps: results,
        // A fixed gap is good news, not a build failure. A different failure
        // means the record is stale and someone has to look at it.
        overall_pass: changed == 0,
    })
}

pub fn print_summary(report: &GapReport, annotate: bool) {
    for item in &report.gaps {
        println!("{:<10} {}: {}", item.status, item.id, item.detail);
        if annotate && item.status != STILL_OPEN && item.status != SKIPPED {
            let title = if item.status == FIXED {
                "Core C gap fixed"
            } else {
                "Core C gap changed"
            };
            println!("::notice title={title}::{}: {}", item.id, item.detail);
        }
    }
    println!(
        "gaps {}: {} still open, {} fixed, {} changed, {} skipped; recorded at {}; overall_pass={}",
        report.gaps_total,
        report.still_open,
        report.fixed,
        report.changed,
        report.skipped,
        report
            .recorded_commit
            .clone()
            .unwrap_or_else(|| "unknown".into()),
        report.overall_pass
    );
    if report.fixed > 0 {
        println!(
            "Promote each fixed gap into tests/selfhost/runtime/cases.json (Codex lane) \
and drop it from conformance/core_c/gaps/gaps.json."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution(exit: i32, stdout: &str, stderr: &str) -> Execution {
        Execution {
            exit: Some(exit),
            stdout: stdout.into(),
            stderr: stderr.into(),
            timed_out: false,
        }
    }

    #[test]
    fn a_gap_recorded_for_one_compiler_is_skipped_by_the_others() {
        let mut clang_only = gap("compile_error");
        clang_only.compilers = vec!["clang".into()];
        assert!(applies_to(&clang_only, "clang-14"));
        assert!(applies_to(&clang_only, "clang"));
        assert!(!applies_to(&clang_only, "x86_64-linux-gnu-gcc-12"));
        // With no compiler recorded, every one of them checks the gap.
        assert!(applies_to(&gap("compile_error"), "x86_64-linux-gnu-gcc-12"));
    }

    fn gap(kind: &str) -> Gap {
        Gap {
            id: "g".into(),
            file: "g.argx".into(),
            kind: kind.into(),
            spec_expected_stdout: "ARGORIX_RESULT:42".into(),
            signature: match kind {
                "emit_rejected" => "checked shifts are not implemented yet".into(),
                "compile_error" => "redefinition of".into(),
                _ => String::new(),
            },
            observed_stdout: (kind == "wrong_result").then(|| "ARGORIX_RESULT:4".to_string()),
            observed_exit: match kind {
                "wrong_result" => Some(0),
                "crash" => Some(139),
                _ => None,
            },
            compilers: Vec::new(),
            note: "note".into(),
        }
    }

    fn outcome() -> Outcome {
        Outcome {
            emit_exit: 0,
            compile_exit: Some(0),
            ..Outcome::default()
        }
    }

    #[test]
    fn recorded_emission_rejection_is_still_open() {
        let observed = Outcome {
            emit_exit: 1,
            emit_stderr: "CBackendUnsupported: checked shifts are not implemented yet".into(),
            ..Outcome::default()
        };
        assert_eq!(classify(&gap("emit_rejected"), &observed).0, STILL_OPEN);
    }

    #[test]
    fn another_emission_message_is_changed() {
        let observed = Outcome {
            emit_exit: 1,
            emit_stderr: "internal error".into(),
            ..Outcome::default()
        };
        assert_eq!(classify(&gap("emit_rejected"), &observed).0, CHANGED);
    }

    #[test]
    fn an_emission_gap_that_now_runs_is_fixed() {
        let observed = Outcome {
            execution: Some(execution(0, "ARGORIX_RESULT:42\n", "")),
            ..outcome()
        };
        assert_eq!(classify(&gap("emit_rejected"), &observed).0, FIXED);
    }

    #[test]
    fn recorded_compile_error_is_still_open() {
        let observed = Outcome {
            compile_exit: Some(1),
            compile_diagnostics: "error: redefinition of `argorix_v_t`".into(),
            ..outcome()
        };
        assert_eq!(classify(&gap("compile_error"), &observed).0, STILL_OPEN);
    }

    #[test]
    fn a_different_compile_error_is_changed() {
        let observed = Outcome {
            compile_exit: Some(1),
            compile_diagnostics: "error: unknown type name".into(),
            ..outcome()
        };
        assert_eq!(classify(&gap("compile_error"), &observed).0, CHANGED);
    }

    #[test]
    fn recorded_wrong_result_is_still_open_and_a_correction_is_fixed() {
        let wrong = Outcome {
            execution: Some(execution(0, "ARGORIX_RESULT:4\n", "")),
            ..outcome()
        };
        assert_eq!(classify(&gap("wrong_result"), &wrong).0, STILL_OPEN);
        let right = Outcome {
            execution: Some(execution(0, "ARGORIX_RESULT:42\n", "")),
            ..outcome()
        };
        assert_eq!(classify(&gap("wrong_result"), &right).0, FIXED);
        let other = Outcome {
            execution: Some(execution(0, "ARGORIX_RESULT:5\n", "")),
            ..outcome()
        };
        assert_eq!(classify(&gap("wrong_result"), &other).0, CHANGED);
    }

    #[test]
    fn a_crash_replaced_by_a_typed_trap_is_fixed() {
        let crash = Outcome {
            execution: Some(execution(139, "", "")),
            ..outcome()
        };
        assert_eq!(classify(&gap("crash"), &crash).0, STILL_OPEN);
        let trap = Outcome {
            execution: Some(execution(70, "", "ARGORIX_TRAP:STEP_LIMIT\n")),
            ..outcome()
        };
        assert_eq!(classify(&gap("crash"), &trap).0, FIXED);
    }

    #[test]
    fn emission_that_starts_failing_is_changed() {
        let observed = Outcome {
            emit_exit: 1,
            emit_stderr: "boom".into(),
            ..Outcome::default()
        };
        assert_eq!(classify(&gap("wrong_result"), &observed).0, CHANGED);
    }
}
