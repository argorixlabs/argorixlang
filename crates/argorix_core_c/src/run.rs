//! The run phase: compile a bundle, execute it, compare, inspect, and report.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::harness::{self, Bundle, Case, CaseResult, Host, ToolRecord, Toolchain, BUNDLE_KIND};
use crate::policy::Policy;

pub const SCOPE: &str = "Transitional C execution of Core 0.1 runtime cases (ESP-008). \
Not self-hosting, not a native backend, not independence of the toolchain from Rust.";

#[derive(Debug, Clone, Serialize)]
pub struct Control {
    pub id: String,
    pub detected: bool,
    pub detail: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub task: String,
    pub scope: String,
    pub commit: Option<String>,
    pub execution_host: Host,
    pub rust_free_host_required: bool,
    pub emission_commit: Option<String>,
    pub emission_host: Host,
    pub emission_argorixc: ToolRecord,
    pub compiler: ToolRecord,
    pub inputs_sha256: Vec<(String, String)>,
    pub cases_total: usize,
    pub cases_passed: usize,
    pub cases: Vec<CaseResult>,
    pub negative_controls: Vec<Control>,
    pub negative_controls_run: bool,
    pub overall_pass: bool,
}

pub struct RunOptions<'a> {
    pub root: &'a Path,
    pub bundle_dir: &'a Path,
    pub cases_path: &'a Path,
    pub compiler_name: &'a str,
    pub work: &'a Path,
    pub with_controls: bool,
    pub require_rust_free: bool,
}

pub fn load_bundle(bundle_dir: &Path, cases_path: &Path) -> Result<Bundle> {
    let text = std::fs::read_to_string(bundle_dir.join("bundle.json"))
        .with_context(|| format!("{} is not an emit bundle", bundle_dir.display()))?;
    let bundle: Bundle = serde_json::from_str(&text)?;
    anyhow::ensure!(
        bundle.schema_version == 1 && bundle.kind == BUNDLE_KIND,
        "{} is not an emit bundle",
        bundle_dir.display()
    );
    anyhow::ensure!(
        bundle.cases_manifest_sha256 == harness::sha256_file(cases_path)?,
        "bundle was emitted from a different cases.json than the one given"
    );
    Ok(bundle)
}

pub fn run(options: RunOptions<'_>) -> Result<Report> {
    let host = harness::host();
    if options.require_rust_free {
        let present: Vec<&str> = host
            .rust_tools_on_path
            .iter()
            .filter(|(_, found)| **found)
            .map(|(name, _)| name.as_str())
            .collect();
        anyhow::ensure!(
            present.is_empty(),
            "--require-rust-free-host, but found on PATH: {}",
            present.join(", ")
        );
    }
    let toolchain: Toolchain = serde_json::from_str(&std::fs::read_to_string(
        options.root.join("bootstrap/c/toolchain.json"),
    )?)?;
    let policy = Policy::load(&options.root.join("conformance/core_c/policy.json"))?;
    let bundle = load_bundle(options.bundle_dir, options.cases_path)?;
    let cases = harness::load_cases(options.cases_path)?;
    let compiler = harness::resolve_compiler(options.compiler_name, &toolchain)?;
    std::fs::create_dir_all(options.work)?;

    let mut results = Vec::new();
    for case in &cases {
        let entry = bundle
            .cases
            .iter()
            .find(|entry| entry.id == case.id)
            .ok_or_else(|| anyhow::anyhow!("bundle has no case {}", case.id))?;
        results.push(run_case(
            case, entry, &options, &toolchain, &policy, &compiler,
        )?);
    }
    let controls = if options.with_controls {
        negative_controls(&results, &cases, options.work, &compiler, &policy)?
    } else {
        Vec::new()
    };

    let passed = results.iter().filter(|item| item.passed).count();
    let controls_ok = controls.iter().all(|control| control.detected);
    let mut inputs = vec![
        (
            harness::relative(options.cases_path, options.root),
            harness::sha256_file(options.cases_path)?,
        ),
        (
            "bootstrap/c/toolchain.json".to_string(),
            harness::sha256_file(&options.root.join("bootstrap/c/toolchain.json"))?,
        ),
        (
            "conformance/core_c/policy.json".to_string(),
            harness::sha256_file(&options.root.join("conformance/core_c/policy.json"))?,
        ),
    ];
    for source in &toolchain.runtime_sources {
        inputs.push((
            source.clone(),
            harness::sha256_file(&options.root.join(source))?,
        ));
    }
    Ok(Report {
        schema_version: 1,
        task: "ESP-008.R".into(),
        scope: SCOPE.into(),
        commit: harness::git_commit(options.root),
        execution_host: host,
        rust_free_host_required: options.require_rust_free,
        emission_commit: bundle.commit.clone(),
        emission_host: bundle.host.clone(),
        emission_argorixc: bundle.argorixc.clone(),
        compiler: ToolRecord {
            path: compiler.display().to_string(),
            sha256: String::new(),
            version: harness::first_line(&compiler, &["--version"]),
        },
        inputs_sha256: inputs,
        cases_total: results.len(),
        cases_passed: passed,
        overall_pass: passed == results.len() && options.with_controls && controls_ok,
        cases: results,
        negative_controls: controls,
        negative_controls_run: options.with_controls,
    })
}

fn run_case(
    case: &Case,
    entry: &crate::harness::BundleEntry,
    options: &RunOptions<'_>,
    toolchain: &Toolchain,
    policy: &Policy,
    compiler: &Path,
) -> Result<CaseResult> {
    let mut result = CaseResult {
        id: case.id.clone(),
        expected: case.clone(),
        c_sha256: entry.c_sha256.clone(),
        compile_argv: None,
        compile_exit: None,
        compile_diagnostics: String::new(),
        executable_sha256: None,
        observed: None,
        dependencies: None,
        failures: Vec::new(),
        passed: false,
    };
    let Some(c_file) = entry.c_file.as_ref() else {
        result
            .failures
            .push(format!("emission failed: {}", entry.emit_stderr));
        return Ok(result);
    };
    if !entry.deterministic {
        result
            .failures
            .push("emission is not deterministic across two runs".into());
    }
    let c_path = options.bundle_dir.join(c_file);
    if Some(harness::sha256_file(&c_path)?) != entry.c_sha256 {
        result
            .failures
            .push("generated C does not match bundle hash".into());
        return Ok(result);
    }

    let executable = options.work.join(&case.id);
    let argv = harness::compile_argv(compiler, toolchain, options.root, &[c_path], &executable)?;
    let (code, _, diagnostics) = harness::run_argv(&argv)?;
    result.compile_argv = Some(
        argv.iter()
            .map(|item| shorten(item, options.root))
            .collect(),
    );
    result.compile_exit = Some(code);
    result.compile_diagnostics = diagnostics.trim().to_string();
    if code != 0 || !executable.is_file() {
        result.failures.push("C compilation failed".into());
        return Ok(result);
    }
    result.executable_sha256 = Some(harness::sha256_file(&executable)?);

    let execution = harness::execute(
        &executable,
        Duration::from_secs(policy.execution_timeout_seconds),
    )?;
    if execution.timed_out {
        result.failures.push(format!(
            "timed out after {} s",
            policy.execution_timeout_seconds
        ));
    } else {
        result.failures.extend(harness::compare(case, &execution));
    }
    result.observed = Some(execution);

    let dependencies = harness::inspect_binary(&executable, policy)?;
    result.failures.extend(dependencies.violations.clone());
    result.dependencies = Some(dependencies);
    result.passed = result.failures.is_empty();
    Ok(result)
}

fn shorten(item: &str, root: &Path) -> String {
    let path = Path::new(item);
    if path.is_absolute() {
        harness::relative(path, root)
    } else {
        item.to_string()
    }
}

// --------------------------------------------------------------------------
// Negative controls: the checks must be able to fail

const SPAWN_SENSOR: &str =
    "#include <stdlib.h>\nint main(void) { return system(\"true\") == 0 ? 0 : 1; }\n";
const SYMBOL_SENSOR: &str =
    "void __rust_alloc(void);\nvoid __rust_alloc(void) {}\nint main(void) { __rust_alloc(); return 0; }\n";
const LIBRARY_SENSOR: &str =
    "int rust_sensor_value(void);\nint rust_sensor_value(void) { return 1; }\n";
const LIBRARY_SENSOR_MAIN: &str =
    "int rust_sensor_value(void);\nint main(void) { return rust_sensor_value(); }\n";

fn sensor_binary(
    name: &str,
    source_text: &str,
    work: &Path,
    compiler: &Path,
    extra: &[String],
) -> Result<PathBuf> {
    // Sensors are compiled, never executed.
    let source = work.join(format!("{name}.c"));
    std::fs::write(&source, source_text)?;
    let output = work.join(name);
    let mut argv = vec![
        compiler.display().to_string(),
        source.display().to_string(),
        "-o".to_string(),
        output.display().to_string(),
    ];
    argv.extend(extra.iter().cloned());
    let (code, _, diagnostics) = harness::run_argv(&argv)?;
    anyhow::ensure!(code == 0, "sensor {name} did not compile: {diagnostics}");
    Ok(output)
}

fn negative_controls(
    results: &[CaseResult],
    cases: &[Case],
    work: &Path,
    compiler: &Path,
    policy: &Policy,
) -> Result<Vec<Control>> {
    let mut controls = Vec::new();
    match results.iter().find(|item| item.observed.is_some()) {
        None => controls.push(Control {
            id: "comparator_detects_mismatch".into(),
            detected: false,
            detail: vec!["no executed case available".into()],
        }),
        Some(executed) => {
            let case = cases
                .iter()
                .find(|case| case.id == executed.id)
                .expect("executed case is in the manifest");
            let mutated = Case {
                expected_stdout: format!("{}_MUTATED", case.expected_stdout),
                expected_exit: case.expected_exit + 1,
                ..case.clone()
            };
            let observed = executed.observed.as_ref().expect("checked above");
            let mismatches = harness::compare(&mutated, observed);
            controls.push(Control {
                id: "comparator_detects_mismatch".into(),
                detected: mismatches.len() == 2,
                detail: mismatches,
            });
        }
    }

    let sensors = work.join("sensors");
    std::fs::create_dir_all(&sensors)?;
    let spawn = harness::inspect_binary(
        &sensor_binary("spawn_sensor", SPAWN_SENSOR, &sensors, compiler, &[])?,
        policy,
    )?;
    controls.push(Control {
        id: "inspector_detects_process_spawn".into(),
        detected: spawn.violations.iter().any(|item| item.contains("system")),
        detail: spawn.violations,
    });
    let symbol = harness::inspect_binary(
        &sensor_binary("symbol_sensor", SYMBOL_SENSOR, &sensors, compiler, &[])?,
        policy,
    )?;
    controls.push(Control {
        id: "inspector_detects_rust_symbol".into(),
        detected: symbol
            .violations
            .iter()
            .any(|item| item.contains("__rust_alloc")),
        detail: symbol.violations,
    });

    let library_source = sensors.join("rust_sensor.c");
    std::fs::write(&library_source, LIBRARY_SENSOR)?;
    let shared = sensors.join("librust_sensor.so");
    let build = vec![
        compiler.display().to_string(),
        "-shared".into(),
        "-fPIC".into(),
        library_source.display().to_string(),
        "-o".into(),
        shared.display().to_string(),
    ];
    let (code, _, diagnostics) = harness::run_argv(&build)?;
    anyhow::ensure!(code == 0, "library sensor did not compile: {diagnostics}");
    let linked = sensor_binary(
        "library_sensor",
        LIBRARY_SENSOR_MAIN,
        &sensors,
        compiler,
        &[
            "-L".to_string(),
            sensors.display().to_string(),
            "-lrust_sensor".to_string(),
        ],
    )?;
    let library = harness::inspect_binary(&linked, policy)?;
    controls.push(Control {
        id: "inspector_detects_rust_library".into(),
        detected: library
            .violations
            .iter()
            .any(|item| item.contains("librust_sensor.so")),
        detail: library.violations,
    });
    Ok(controls)
}

pub fn print_summary(report: &Report) {
    for item in &report.cases {
        println!("{} {}", if item.passed { "PASS" } else { "FAIL" }, item.id);
        for failure in &item.failures {
            println!("     {failure}");
        }
    }
    for control in &report.negative_controls {
        println!(
            "{} negative control {}",
            if control.detected {
                "DETECTED"
            } else {
                "MISSED"
            },
            control.id
        );
    }
    println!(
        "cases {}/{}; compiler {}; overall_pass={}",
        report.cases_passed, report.cases_total, report.compiler.version, report.overall_pass
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scope_line_refuses_to_overclaim() {
        assert!(SCOPE.contains("Not self-hosting"));
        assert!(SCOPE.contains("not independence"));
    }
}
