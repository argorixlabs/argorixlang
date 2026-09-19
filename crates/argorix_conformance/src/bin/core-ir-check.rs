use anyhow::{bail, Context, Result};
use argorix_ir::core::{CoreIrEffect, CoreIrExpr, CoreIrItemKind, CoreIrProgram, CoreIrStatement};
use argorix_ir::{lower_core_program, verify_core_ir};
use argorix_parser::core::parse_core_source;
use argorix_semantics::{verify_core_program, CoreCheckOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize)]
struct CaseManifest {
    valid_cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
}

#[derive(Serialize)]
struct ModuleResult {
    id: String,
    file: String,
    module: String,
    item_count: usize,
    fingerprint_stable: bool,
    passed: bool,
}

#[derive(Serialize)]
struct MutationResult {
    name: &'static str,
    expected_code: &'static str,
    observed_codes: Vec<String>,
    rejected_as_expected: bool,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    task: &'static str,
    baseline_commit: String,
    scope: &'static str,
    cases_sha256: String,
    ir_implementation_sha256: String,
    ir_schema_sha256: String,
    valid_modules: Vec<ModuleResult>,
    mutation_controls: Vec<MutationResult>,
    accepted_valid: usize,
    rejected_mutations: usize,
    overall_pass: bool,
    not_proven: Vec<&'static str>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn codes(program: &CoreIrProgram) -> Vec<String> {
    let mut result = verify_core_ir(program)
        .expect_err("mutated Core IR was accepted")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    result.sort();
    result.dedup();
    result
}

fn mutation(
    name: &'static str,
    expected_code: &'static str,
    program: &CoreIrProgram,
) -> MutationResult {
    let observed_codes = codes(program);
    MutationResult {
        name,
        expected_code,
        rejected_as_expected: observed_codes.iter().any(|code| code == expected_code),
        observed_codes,
    }
}

fn mutate_first_call(expr: &mut CoreIrExpr) -> bool {
    match expr {
        CoreIrExpr::Call { callee, .. } => {
            **callee = CoreIrExpr::Path {
                segments: vec!["missing_function".into()],
            };
            true
        }
        CoreIrExpr::Block { body } | CoreIrExpr::Loop { body } => mutate_block_calls(body),
        CoreIrExpr::If {
            condition,
            then_block,
            else_expr,
        } => {
            mutate_first_call(condition)
                || mutate_block_calls(then_block)
                || else_expr
                    .as_mut()
                    .is_some_and(|value| mutate_first_call(value))
        }
        CoreIrExpr::Match { value, arms } => {
            mutate_first_call(value)
                || arms.iter_mut().any(|arm| {
                    arm.guard.as_mut().is_some_and(mutate_first_call)
                        || mutate_first_call(&mut arm.value)
                })
        }
        CoreIrExpr::Aggregate { fields, .. } => fields
            .iter_mut()
            .any(|field| mutate_first_call(&mut field.value)),
        CoreIrExpr::Array { values } => values.iter_mut().any(mutate_first_call),
        CoreIrExpr::Field { value, .. } | CoreIrExpr::Unary { value, .. } => {
            mutate_first_call(value)
        }
        CoreIrExpr::Index { value, index } => mutate_first_call(value) || mutate_first_call(index),
        CoreIrExpr::Binary { left, right, .. } => {
            mutate_first_call(left) || mutate_first_call(right)
        }
        CoreIrExpr::Integer { .. }
        | CoreIrExpr::String { .. }
        | CoreIrExpr::Bool { .. }
        | CoreIrExpr::Unit
        | CoreIrExpr::Path { .. } => false,
    }
}

fn mutate_block_calls(block: &mut argorix_ir::core::CoreIrBlock) -> bool {
    for statement in &mut block.statements {
        let found = match statement {
            CoreIrStatement::Let { value, .. }
            | CoreIrStatement::Return { value: Some(value) }
            | CoreIrStatement::Break { value: Some(value) }
            | CoreIrStatement::Expr { value } => mutate_first_call(value),
            CoreIrStatement::Assign { target, value, .. } => {
                mutate_first_call(target) || mutate_first_call(value)
            }
            CoreIrStatement::While { condition, body } => {
                mutate_first_call(condition) || mutate_block_calls(body)
            }
            CoreIrStatement::Break { value: None }
            | CoreIrStatement::Continue
            | CoreIrStatement::Return { value: None } => false,
        };
        if found {
            return true;
        }
    }
    block
        .tail
        .as_mut()
        .is_some_and(|value| mutate_first_call(value))
}

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let cases_path = PathBuf::from(
        args.next()
            .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?,
    );
    let fixture_dir = PathBuf::from(
        args.next()
            .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?,
    );
    let ir_source_path = PathBuf::from(
        args.next()
            .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?,
    );
    let schema_path = PathBuf::from(
        args.next()
            .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?,
    );
    let baseline_commit = args
        .next()
        .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?
        .to_string_lossy()
        .into_owned();
    let output = PathBuf::from(
        args.next()
            .context("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT")?,
    );
    if args.next().is_some() {
        bail!("usage: core-ir-check CASES FIXTURE_DIR IR_SOURCE SCHEMA BASELINE OUTPUT");
    }

    let cases_raw = fs::read(&cases_path)?;
    let ir_source = fs::read(&ir_source_path)?;
    let schema_raw = fs::read(&schema_path)?;
    let _: serde_json::Value =
        serde_json::from_slice(&schema_raw).context("invalid IR schema JSON")?;
    let manifest: CaseManifest = serde_json::from_slice(&cases_raw)?;
    let parsed = manifest
        .valid_cases
        .iter()
        .map(|case| {
            let source = fs::read_to_string(fixture_dir.join(&case.file))?;
            let program = parse_core_source(&source)
                .map_err(|errors| anyhow::anyhow!("{} did not parse: {errors:?}", case.file))?;
            Ok((case, program))
        })
        .collect::<Result<Vec<_>>>()?;
    let modules = parsed
        .iter()
        .map(|(_, program)| program.module.value.clone())
        .collect::<BTreeSet<_>>();

    let mut valid_modules = Vec::new();
    let mut lowered = Vec::new();
    for (case, program) in parsed {
        let checked = verify_core_program(
            &program,
            &CoreCheckOptions {
                available_modules: modules.clone(),
            },
        )
        .map_err(|errors| anyhow::anyhow!("{} failed semantics: {errors:?}", case.file))?;
        let ir = lower_core_program(checked);
        let before = verify_core_ir(&ir)
            .map_err(|errors| anyhow::anyhow!("lowered IR failed verification: {errors:?}"))?
            .semantic_fingerprint();
        let json = serde_json::to_string_pretty(&ir)?;
        let decoded: CoreIrProgram = serde_json::from_str(&json)?;
        let after = verify_core_ir(&decoded)
            .map_err(|errors| anyhow::anyhow!("roundtripped IR failed verification: {errors:?}"))?
            .semantic_fingerprint();
        let fingerprint_stable = before == after;
        valid_modules.push(ModuleResult {
            id: case.id.clone(),
            file: case.file.clone(),
            module: ir.module.clone(),
            item_count: ir.items.len(),
            fingerprint_stable,
            passed: fingerprint_stable,
        });
        lowered.push(ir);
    }

    let base = lowered.first().context("no valid Core cases")?;
    let mut bad_version = base.clone();
    bad_version.ir_version = "9.9".into();
    let mut bad_effect = base.clone();
    bad_effect
        .effect_policy
        .push(CoreIrEffect::Host("process".into()));
    let mut bad_reference = base.clone();
    let changed = bad_reference
        .items
        .iter_mut()
        .any(|item| match &mut item.kind {
            CoreIrItemKind::Function(function) => mutate_block_calls(&mut function.body),
            _ => false,
        });
    if !changed {
        bail!("reference mutation found no call");
    }
    let mut bad_control = base.clone();
    let function = bad_control
        .items
        .iter_mut()
        .find_map(|item| match &mut item.kind {
            CoreIrItemKind::Function(function) => Some(function),
            _ => None,
        })
        .context("control mutation found no function")?;
    function
        .body
        .statements
        .insert(0, CoreIrStatement::Break { value: None });
    let mut bad_type = base.clone();
    let function = bad_type
        .items
        .iter_mut()
        .find_map(|item| match &mut item.kind {
            CoreIrItemKind::Function(function) => Some(function),
            _ => None,
        })
        .context("type mutation found no function")?;
    function.return_type = argorix_ir::core::CoreIrType::Named {
        name: "String".into(),
    };

    let mutation_controls = vec![
        mutation("unknown_ir_version", "IrVersionUnsupported", &bad_version),
        mutation(
            "unauthorized_host_effect",
            "UnauthorizedEffect",
            &bad_effect,
        ),
        mutation(
            "missing_function_reference",
            "ImmutableAssignmentOrUnknownName",
            &bad_reference,
        ),
        mutation("control_outside_loop", "ControlOutsideLoop", &bad_control),
        mutation("return_type_mismatch", "TypeMismatch", &bad_type),
    ];
    let accepted_valid = valid_modules.iter().filter(|case| case.passed).count();
    let rejected_mutations = mutation_controls
        .iter()
        .filter(|case| case.rejected_as_expected)
        .count();
    let overall_pass = accepted_valid == valid_modules.len()
        && valid_modules.len() == 4
        && rejected_mutations == mutation_controls.len();
    let report = Report {
        schema_version: 1,
        task: "ESP-007",
        baseline_commit,
        scope: "Rust stage0 Core 0.1 lowering, serialization, and verification",
        cases_sha256: digest(&cases_raw),
        ir_implementation_sha256: digest(&ir_source),
        ir_schema_sha256: digest(&schema_raw),
        valid_modules,
        mutation_controls,
        accepted_valid,
        rejected_mutations,
        overall_pass,
        not_proven: vec![
            "Core execution or C emission",
            "self-hosted lowering or verifier",
            "native backend",
            "Rust independence",
            "production security or performance",
        ],
    };
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    println!(
        "ESP-007: valid={}/{} mutations={}/{} overall_pass={}",
        report.accepted_valid,
        report.valid_modules.len(),
        report.rejected_mutations,
        report.mutation_controls.len(),
        report.overall_pass
    );
    if !report.overall_pass {
        bail!("ESP-007 Core IR validation failed");
    }
    Ok(())
}
