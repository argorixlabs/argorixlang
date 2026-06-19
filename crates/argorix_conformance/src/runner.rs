use crate::{
    mutation::apply_mutation,
    resolve_fixture_path,
    types::{
        ConformanceCase, ConformanceCaseResult, ConformanceFailure, ConformanceResult,
        ConformanceStageResult, ConformanceSuite,
    },
    validate_suite,
    workspace::CaseWorkspace,
    ConformanceError,
};
use argorix_bytecode::{lower_ir, verify_bytecode, BytecodeProgram};
use argorix_ir::IrProgram;
use argorix_parser::{parse_source, Program};
use argorix_semantics::check_program;
use argorix_vm::{
    evidence::{verify_evidence, EvidenceBundle},
    parse_injection, ExecutionOutcome, SecurityReport, Vm,
};
use std::{fs, path::Path};

pub fn run_suite(
    suite: &ConformanceSuite,
    suite_path: &Path,
    workdir: &Path,
) -> Result<ConformanceResult, ConformanceError> {
    validate_suite(suite, suite_path)?;
    let mut case_results = Vec::with_capacity(suite.cases.len());
    let mut failures = Vec::new();
    for case in &suite.cases {
        let (result, failure) = run_case(case, suite_path, workdir)?;
        case_results.push(result);
        if let Some(failure) = failure {
            failures.push(failure);
        }
    }
    let cases_passed = case_results.iter().filter(|case| case.passed).count();
    let cases_total = case_results.len();
    Ok(ConformanceResult {
        suite_version: suite.suite_version.clone(),
        passed: cases_passed == cases_total,
        cases_total,
        cases_passed,
        cases_failed: cases_total - cases_passed,
        case_results,
        failures,
    })
}

fn run_case(
    case: &ConformanceCase,
    suite_path: &Path,
    workdir: &Path,
) -> Result<(ConformanceCaseResult, Option<ConformanceFailure>), ConformanceError> {
    let workspace = CaseWorkspace::create(workdir, &case.id)?;
    let mut state = CaseState::default();
    let mut stage_results = Vec::with_capacity(case.stages.len());
    let mut failure = None;
    let mut case_passed = true;
    let mut stopped = false;

    for stage in &case.stages {
        if stopped {
            stage_results.push(ConformanceStageResult {
                stage: stage.clone(),
                status: "skipped".into(),
                message: None,
            });
            continue;
        }

        let stage_execution = if case
            .mutation
            .as_ref()
            .is_some_and(|mutation| mutation.before_stage == *stage)
        {
            let mutation = case.mutation.as_ref().unwrap();
            apply_mutation(mutation, &workspace)
                .and_then(|_| reload_mutated_artifact(mutation, &workspace, &mut state))
                .and_then(|_| execute_stage(stage, case, suite_path, &workspace, &mut state))
        } else {
            execute_stage(stage, case, suite_path, &workspace, &mut state)
        };

        match stage_execution {
            Ok(message) => {
                stage_results.push(ConformanceStageResult {
                    stage: stage.clone(),
                    status: "passed".into(),
                    message,
                });
                if case.expected_failure_stage.as_deref() == Some(stage.as_str()) {
                    let reason = format!("expected stage `{stage}` to fail, but it passed");
                    case_passed = false;
                    failure = Some(ConformanceFailure {
                        case_id: case.id.clone(),
                        stage: stage.clone(),
                        reason,
                    });
                    stopped = true;
                }
            }
            Err(reason) => {
                stage_results.push(ConformanceStageResult {
                    stage: stage.clone(),
                    status: "failed".into(),
                    message: Some(reason.clone()),
                });
                let expected_stage = case.expected_failure_stage.as_deref() == Some(stage.as_str());
                let expected_message = case
                    .expected_failure_contains
                    .as_deref()
                    .is_none_or(|expected| reason.contains(expected));
                if expected_stage && expected_message {
                    case_passed = true;
                } else {
                    case_passed = false;
                    let mismatch = if expected_stage {
                        format!(
                            "failure did not contain expected text `{}`: {reason}",
                            case.expected_failure_contains.as_deref().unwrap_or("")
                        )
                    } else {
                        reason
                    };
                    failure = Some(ConformanceFailure {
                        case_id: case.id.clone(),
                        stage: stage.clone(),
                        reason: mismatch,
                    });
                }
                stopped = true;
            }
        }
    }

    Ok((
        ConformanceCaseResult {
            id: case.id.clone(),
            name: case.name.clone(),
            category: case.category.clone(),
            passed: case_passed,
            stages: stage_results,
        },
        failure,
    ))
}

fn reload_mutated_artifact(
    mutation: &crate::types::ConformanceMutation,
    workspace: &CaseWorkspace,
    state: &mut CaseState,
) -> Result<(), String> {
    if mutation.artifact == "bytecode" {
        let source = fs::read_to_string(&workspace.bytecode)
            .map_err(|error| format!("failed to reload mutated Bytecode: {error}"))?;
        state.bytecode = Some(
            serde_json::from_str(&source)
                .map_err(|error| format!("mutated Bytecode is invalid JSON: {error}"))?,
        );
    } else if mutation.artifact == "security_report" {
        let source = fs::read_to_string(&workspace.report)
            .map_err(|error| format!("failed to reload mutated SecurityReport: {error}"))?;
        state.report = Some(
            serde_json::from_str(&source)
                .map_err(|error| format!("mutated SecurityReport is invalid JSON: {error}"))?,
        );
    }
    Ok(())
}

fn execute_stage(
    stage: &str,
    case: &ConformanceCase,
    suite_path: &Path,
    workspace: &CaseWorkspace,
    state: &mut CaseState,
) -> Result<Option<String>, String> {
    match stage {
        "parse" => {
            let path = resolve_fixture_path(
                suite_path,
                case.source_path
                    .as_deref()
                    .ok_or_else(|| "source_path is required".to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read source fixture: {error}"))?;
            let program = parse_source(&source).map_err(diagnostic_messages)?;
            state.program = Some(program);
            Ok(None)
        }
        "semantic_check" => {
            let program = state
                .program
                .as_ref()
                .ok_or_else(|| "parse output is unavailable".to_string())?;
            check_program(program).map_err(diagnostic_messages)?;
            Ok(None)
        }
        "emit_ir" => {
            let program = state
                .program
                .as_ref()
                .ok_or_else(|| "validated program is unavailable".to_string())?;
            let ir = IrProgram::from(program);
            workspace
                .write_json(&workspace.ir, &ir)
                .map_err(|error| error.to_string())?;
            state.ir = Some(ir);
            Ok(None)
        }
        "emit_bytecode" => {
            let ir = state
                .ir
                .as_ref()
                .ok_or_else(|| "IR is unavailable".to_string())?;
            let bytecode = lower_ir(ir);
            workspace
                .write_json(&workspace.bytecode, &bytecode)
                .map_err(|error| error.to_string())?;
            state.bytecode = Some(bytecode);
            Ok(None)
        }
        "verify_bytecode" => {
            if state.bytecode.is_none() {
                let path = resolve_fixture_path(
                    suite_path,
                    case.bytecode_path
                        .as_deref()
                        .ok_or_else(|| "bytecode_path is required".to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let source = fs::read_to_string(path)
                    .map_err(|error| format!("failed to read Bytecode fixture: {error}"))?;
                let bytecode: BytecodeProgram = serde_json::from_str(&source)
                    .map_err(|error| format!("invalid Bytecode JSON: {error}"))?;
                workspace
                    .write_json(&workspace.bytecode, &bytecode)
                    .map_err(|error| error.to_string())?;
                state.bytecode = Some(bytecode);
            }
            verify_bytecode(state.bytecode.as_ref().unwrap()).map_err(|errors| {
                join_messages(errors.into_iter().map(|error| error.to_string()))
            })?;
            Ok(None)
        }
        "run_vm" => {
            let bytecode = state
                .bytecode
                .as_ref()
                .ok_or_else(|| "verified Bytecode is unavailable".to_string())?;
            let injection = parse_injection(
                case.injection
                    .as_deref()
                    .ok_or_else(|| "injection is required".to_string())?,
            )
            .map_err(|error| error.to_string())?;
            state.outcome = Some(Vm::new().run_reactive_outcome(bytecode, injection));
            Ok(None)
        }
        "security_report" => {
            let bytecode = state
                .bytecode
                .as_ref()
                .ok_or_else(|| "Bytecode is unavailable".to_string())?;
            let outcome = state
                .outcome
                .as_ref()
                .ok_or_else(|| "VM outcome is unavailable".to_string())?;
            let report = SecurityReport::from_outcome(bytecode, outcome);
            workspace
                .write_json(&workspace.report, &report)
                .map_err(|error| error.to_string())?;
            state.report = Some(report);
            Ok(None)
        }
        "trace_out" => {
            let outcome = state
                .outcome
                .as_ref()
                .ok_or_else(|| "VM outcome is unavailable".to_string())?;
            match outcome.result.as_ref() {
                Ok(trace) => {
                    workspace
                        .write_json(&workspace.trace, trace)
                        .map_err(|error| error.to_string())?;
                    Ok(None)
                }
                Err(_) => Ok(Some("trace unavailable for failed execution".into())),
            }
        }
        "evidence_bundle" => {
            let bytecode = state
                .bytecode
                .as_ref()
                .ok_or_else(|| "Bytecode is unavailable".to_string())?;
            let outcome = state
                .outcome
                .as_ref()
                .ok_or_else(|| "VM outcome is unavailable".to_string())?;
            let report = state
                .report
                .as_ref()
                .ok_or_else(|| "SecurityReport is unavailable".to_string())?;
            let trace_path = workspace
                .trace
                .exists()
                .then_some(workspace.trace.as_path());
            let bundle = EvidenceBundle::from_outcome(
                bytecode,
                outcome,
                report,
                &workspace.bundle,
                Some(&workspace.bytecode),
                trace_path,
                Some(&workspace.report),
            )
            .map_err(|error| error.to_string())?;
            workspace
                .write_json(&workspace.bundle, &bundle)
                .map_err(|error| error.to_string())?;
            Ok(None)
        }
        "verify_evidence" => {
            let verification =
                verify_evidence(&workspace.bundle).map_err(|error| error.to_string())?;
            if verification.passed {
                Ok(None)
            } else {
                Err(verification.failures.join("; "))
            }
        }
        other => Err(format!("unsupported stage `{other}`")),
    }
}

#[derive(Default)]
struct CaseState {
    program: Option<Program>,
    ir: Option<IrProgram>,
    bytecode: Option<BytecodeProgram>,
    outcome: Option<ExecutionOutcome>,
    report: Option<SecurityReport>,
}

fn diagnostic_messages(diagnostics: Vec<argorix_parser::Diagnostic>) -> String {
    join_messages(diagnostics.into_iter().map(|diagnostic| diagnostic.message))
}

fn join_messages(messages: impl Iterator<Item = String>) -> String {
    messages.collect::<Vec<_>>().join("; ")
}
