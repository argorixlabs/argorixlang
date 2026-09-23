//! Emit, compile, execute, compare, and inspect the Core runtime cases.
//!
//! The case manifest is the oracle; `argorixc core-emit-c`, the C compiler and
//! the C1 runtime are the implementations under test. Commands are always an
//! argument vector, never a shell string, and case contents never become
//! arguments.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::elf;
use crate::policy::Policy;

pub const BUNDLE_KIND: &str = "argorix-core-c-emit-bundle";

#[derive(Debug, Clone, Deserialize)]
pub struct CasesManifest {
    pub schema_version: u32,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Case {
    pub id: String,
    pub file: String,
    pub expected_exit: i32,
    pub expected_stdout: String,
    pub expected_stderr: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Toolchain {
    pub allowed_compiler_names: Vec<String>,
    pub required_flags: Vec<String>,
    pub runtime_sources: Vec<String>,
    #[serde(default)]
    pub forbidden_link_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEntry {
    pub id: String,
    pub source: String,
    pub source_sha256: String,
    pub emit_exit: i32,
    pub emit_stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_sha256: Option<String>,
    pub deterministic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub schema_version: u32,
    pub kind: String,
    pub commit: Option<String>,
    pub host: Host,
    pub argorixc: ToolRecord,
    pub cases_manifest: String,
    pub cases_manifest_sha256: String,
    pub cases: Vec<BundleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecord {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub os: String,
    pub arch: String,
    pub rust_tools_on_path: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Execution {
    pub exit: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub expected: Case,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_exit: Option<i32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub compile_diagnostics: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<Execution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DependencyReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sanitizer_findings: Vec<String>,
    pub failures: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyReport {
    pub format: String,
    pub needed: Vec<String>,
    pub imported_symbols: Vec<String>,
    pub symbol_count: usize,
    pub violations: Vec<String>,
    pub passed: bool,
}

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(sha256_bytes(&data))
}

pub fn host() -> Host {
    let mut tools = BTreeMap::new();
    for tool in ["rustc", "cargo", "rustup"] {
        tools.insert(tool.to_string(), which(tool).is_some());
    }
    Host {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        rust_tools_on_path: tools,
    }
}

/// Look up an executable on PATH without spawning a shell.
pub fn which(name: &str) -> Option<PathBuf> {
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE".into())
            .split(';')
            .map(|item| item.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = directory.join(format!("{name}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn first_line(program: &Path, args: &[&str]) -> String {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let text = if output.stdout.is_empty() {
                output.stderr
            } else {
                output.stdout
            };
            String::from_utf8_lossy(&text)
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        }
        Err(error) => format!("unavailable: {error}"),
    }
}

pub fn git_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &root.display().to_string(), "rev-parse", "HEAD"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub fn decode(data: &[u8]) -> String {
    String::from_utf8_lossy(data).replace("\r\n", "\n")
}

pub fn strip_one_newline(text: &str) -> &str {
    text.strip_suffix('\n').unwrap_or(text)
}

// --------------------------------------------------------------------------
// Case manifest

pub fn load_cases(path: &Path) -> Result<Vec<Case>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest: CasesManifest = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    anyhow::ensure!(
        manifest.schema_version == 1,
        "unsupported cases schema_version"
    );
    anyhow::ensure!(
        !manifest.cases.is_empty(),
        "{} declares no cases",
        path.display()
    );
    let base = path.parent().unwrap_or(Path::new("."));
    let mut seen = Vec::new();
    for case in &manifest.cases {
        validate_case(case, base)?;
        anyhow::ensure!(!seen.contains(&case.id), "duplicate case id: {}", case.id);
        seen.push(case.id.clone());
    }
    Ok(manifest.cases)
}

pub fn validate_case(case: &Case, base: &Path) -> Result<()> {
    anyhow::ensure!(
        !case.id.is_empty()
            && case
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
        "case id `{}` must be lowercase letters, digits, or underscore",
        case.id
    );
    anyhow::ensure!(
        !case.file.contains("..")
            && !case.file.contains('\\')
            && !case.file.starts_with('/')
            && case.file.ends_with(".argx"),
        "case {}: file must be a plain relative .argx path",
        case.id
    );
    anyhow::ensure!(
        base.join(&case.file).is_file(),
        "case {}: missing source {}",
        case.id,
        base.join(&case.file).display()
    );
    Ok(())
}

// --------------------------------------------------------------------------
// Emission

pub fn emitter_available(argorixc: &Path) -> Result<bool> {
    let output = Command::new(argorixc)
        .arg("--help")
        .output()
        .with_context(|| format!("cannot execute argorixc at {}", argorixc.display()))?;
    let text = decode(&output.stdout) + &decode(&output.stderr);
    Ok(text.contains("core-emit-c"))
}

pub fn emit_once(
    argorixc: &Path,
    root: &Path,
    source: &Path,
    output: &Path,
) -> Result<(i32, String)> {
    let mut command = Command::new(argorixc);
    // The Core standard library lives in `stdlib/` of the repository; the
    // driver only reads it when told to, so the harness tells it.
    let stdlib = root.join("stdlib");
    if stdlib.is_dir() {
        command.arg("--stdlib").arg(stdlib);
    }
    let result = command
        .arg("core-emit-c")
        .arg(source)
        .arg("--output")
        .arg(output)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run {}", argorixc.display()))?;
    Ok((
        result.status.code().unwrap_or(-1),
        decode(&result.stderr).trim().to_string(),
    ))
}

pub fn emit(argorixc: &Path, root: &Path, cases_path: &Path, bundle_dir: &Path) -> Result<Bundle> {
    anyhow::ensure!(
        argorixc.is_file(),
        "argorixc not found: {}",
        argorixc.display()
    );
    anyhow::ensure!(
        emitter_available(argorixc)?,
        "argorixc has no `core-emit-c` command; the ESP-008 emitter is not in this build"
    );
    let cases = load_cases(cases_path)?;
    std::fs::create_dir_all(bundle_dir)?;
    let base = cases_path.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    for case in &cases {
        let source = base.join(&case.file);
        let first = bundle_dir.join(format!("{}.c", case.id));
        let repeat = bundle_dir.join(format!("{}.c.repeat", case.id));
        let (code, stderr) = emit_once(argorixc, root, &source, &first)?;
        let mut entry = BundleEntry {
            id: case.id.clone(),
            source: relative(&source, root),
            source_sha256: sha256_file(&source)?,
            emit_exit: code,
            emit_stderr: stderr,
            c_file: None,
            c_sha256: None,
            deterministic: false,
        };
        if code == 0 && first.is_file() {
            let digest = sha256_file(&first)?;
            let (repeat_code, _) = emit_once(argorixc, root, &source, &repeat)?;
            entry.deterministic =
                repeat_code == 0 && repeat.is_file() && sha256_file(&repeat)? == digest;
            let _ = std::fs::remove_file(&repeat);
            entry.c_file = Some(format!("{}.c", case.id));
            entry.c_sha256 = Some(digest);
        }
        entries.push(entry);
    }
    let bundle = Bundle {
        schema_version: 1,
        kind: BUNDLE_KIND.to_string(),
        commit: git_commit(root),
        host: host(),
        argorixc: ToolRecord {
            path: relative(argorixc, root),
            sha256: sha256_file(argorixc)?,
            version: first_line(argorixc, &["--version"]),
        },
        cases_manifest: relative(cases_path, root),
        cases_manifest_sha256: sha256_file(cases_path)?,
        cases: entries,
    };
    std::fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_string_pretty(&bundle)? + "\n",
    )?;
    Ok(bundle)
}

pub fn relative(path: &Path, root: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let base = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    match canonical.strip_prefix(&base) {
        Ok(rest) => rest.to_string_lossy().replace('\\', "/"),
        Err(_) => canonical.to_string_lossy().replace('\\', "/"),
    }
}

// --------------------------------------------------------------------------
// Compilation

pub fn resolve_compiler(requested: &str, toolchain: &Toolchain) -> Result<PathBuf> {
    let candidate = Path::new(requested);
    if candidate.is_absolute() {
        // An explicitly configured absolute path, as toolchain.json allows.
        anyhow::ensure!(candidate.is_file(), "compiler is not a file: {requested}");
        let stem = candidate
            .file_stem()
            .map(|item| item.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        anyhow::ensure!(
            stem != "rustc" && stem != "cargo",
            "compiler path names a Rust tool"
        );
        return Ok(candidate.to_path_buf());
    }
    anyhow::ensure!(
        requested.chars().all(|c| c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || c == '+'
            || c == '_')
            && toolchain
                .allowed_compiler_names
                .iter()
                .any(|name| name == requested),
        "compiler `{requested}` is not allow-listed; use one of {:?} or an absolute path",
        toolchain.allowed_compiler_names
    );
    which(requested).ok_or_else(|| anyhow::anyhow!("compiler `{requested}` is not on PATH"))
}

pub fn check_argv(argv: &[String], toolchain: &Toolchain) -> Result<()> {
    for item in argv {
        let name = Path::new(item)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| item.clone());
        for pattern in &toolchain.forbidden_link_inputs {
            if glob_matches(pattern, &name) {
                bail!("compile argument `{item}` matches forbidden input `{pattern}`");
            }
        }
    }
    Ok(())
}

/// `*` wildcards only, which is all `forbidden_link_inputs` uses.
pub fn glob_matches(pattern: &str, value: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == value,
        Some((prefix, suffix)) => {
            value.len() >= prefix.len() + suffix.len()
                && value.starts_with(prefix)
                && value.ends_with(suffix)
        }
    }
}

/// The sanitizers the `sanitize` mode adds on top of the declared profile.
///
/// `-fno-sanitize-recover=all` makes undefined behaviour abort instead of
/// printing and continuing, so a finding cannot be missed.
pub const SANITIZER_FLAGS: [&str; 3] = [
    "-fsanitize=address,undefined",
    "-fno-sanitize-recover=all",
    "-g",
];

pub fn compile_argv(
    compiler: &Path,
    toolchain: &Toolchain,
    root: &Path,
    sources: &[PathBuf],
    output: &Path,
) -> Result<Vec<String>> {
    compile_argv_with(compiler, toolchain, root, sources, output, &[])
}

pub fn compile_argv_with(
    compiler: &Path,
    toolchain: &Toolchain,
    root: &Path,
    sources: &[PathBuf],
    output: &Path,
    extra_flags: &[&str],
) -> Result<Vec<String>> {
    anyhow::ensure!(
        !toolchain.required_flags.is_empty(),
        "toolchain.json declares no required_flags"
    );
    let mut argv = vec![compiler.display().to_string()];
    argv.extend(toolchain.required_flags.iter().cloned());
    argv.extend(extra_flags.iter().map(|flag| flag.to_string()));
    let mut includes: Vec<String> = toolchain
        .runtime_sources
        .iter()
        .filter_map(|item| {
            root.join(item)
                .parent()
                .map(|parent| parent.display().to_string())
        })
        .collect();
    includes.sort();
    includes.dedup();
    for directory in includes {
        argv.push("-I".into());
        argv.push(directory);
    }
    for source in sources {
        argv.push(source.display().to_string());
    }
    for item in toolchain
        .runtime_sources
        .iter()
        .filter(|item| item.ends_with(".c"))
    {
        argv.push(root.join(item).display().to_string());
    }
    argv.push("-o".into());
    argv.push(output.display().to_string());
    check_argv(&argv, toolchain)?;
    Ok(argv)
}

pub fn run_argv(argv: &[String]) -> Result<(i32, String, String)> {
    let output = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to run {}", argv[0]))?;
    Ok((
        output.status.code().unwrap_or(-1),
        decode(&output.stdout),
        decode(&output.stderr),
    ))
}

// --------------------------------------------------------------------------
// Execution

pub fn execute(executable: &Path, timeout: Duration) -> Result<Execution> {
    execute_with_env(executable, timeout, &[])
}

/// Lines that mean a sanitizer caught something, whatever the exit status.
pub fn sanitizer_findings(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| {
            line.contains("LeakSanitizer")
                || line.contains("AddressSanitizer:")
                || line.contains("runtime error:")
        })
        .map(|line| line.trim().to_string())
        .collect()
}

pub fn execute_with_env(
    executable: &Path,
    timeout: Duration,
    env: &[(&str, &str)],
) -> Result<Execution> {
    let mut command = Command::new(executable);
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {}", executable.display()))?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            return Ok(Execution {
                exit: status.code().or_else(|| signal_exit(&status)),
                stdout: decode(&output.stdout),
                stderr: decode(&output.stderr),
                timed_out: false,
            });
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Execution {
                exit: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn signal_exit(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    // A signalled child reports 128 + signal, the value a shell would show.
    status.signal().map(|signal| 128 + signal)
}

#[cfg(not(unix))]
fn signal_exit(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

pub fn compare(case: &Case, execution: &Execution) -> Vec<String> {
    let mut mismatches = Vec::new();
    match execution.exit {
        Some(code) if code == case.expected_exit => {}
        Some(code) => mismatches.push(format!("exit {code} != expected {}", case.expected_exit)),
        None => mismatches.push(format!("no exit status, expected {}", case.expected_exit)),
    }
    let stdout = strip_one_newline(&execution.stdout);
    if stdout != case.expected_stdout {
        mismatches.push(format!(
            "stdout {stdout:?} != expected {:?}",
            case.expected_stdout
        ));
    }
    let stderr = strip_one_newline(&execution.stderr);
    if stderr != case.expected_stderr {
        mismatches.push(format!(
            "stderr {stderr:?} != expected {:?}",
            case.expected_stderr
        ));
    }
    mismatches
}

pub fn inspect_binary(executable: &Path, policy: &Policy) -> Result<DependencyReport> {
    let content = std::fs::read(executable)?;
    if !elf::is_elf(&content) {
        return Ok(DependencyReport {
            format: "not-elf".into(),
            needed: Vec::new(),
            imported_symbols: Vec::new(),
            symbol_count: 0,
            violations: vec!["dependency inspection supports Linux ELF only".into()],
            passed: false,
        });
    }
    let inspection = elf::inspect(&content)?;
    let violations = policy.violations(&inspection, &content);
    Ok(DependencyReport {
        format: "elf".into(),
        needed: inspection.needed.clone(),
        imported_symbols: inspection
            .imported()
            .into_iter()
            .map(str::to_string)
            .collect(),
        symbol_count: inspection.symbols.len(),
        passed: violations.is_empty(),
        violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toolchain() -> Toolchain {
        Toolchain {
            allowed_compiler_names: vec!["cc".into(), "clang".into(), "gcc".into()],
            required_flags: vec![
                "-std=c11".into(),
                "-Wall".into(),
                "-Wextra".into(),
                "-Werror".into(),
            ],
            runtime_sources: vec![
                "bootstrap/c/argorix_core_runtime.c".into(),
                "bootstrap/c/argorix_core_runtime.h".into(),
            ],
            forbidden_link_inputs: vec![
                "*.rlib".into(),
                "*.rustlib".into(),
                "cargo".into(),
                "rustc".into(),
            ],
        }
    }

    fn case() -> Case {
        Case {
            id: "scalar_success".into(),
            file: "scalar_success.argx".into(),
            expected_exit: 0,
            expected_stdout: "ARGORIX_RESULT:42".into(),
            expected_stderr: String::new(),
        }
    }

    fn execution(exit: i32, stdout: &str, stderr: &str) -> Execution {
        Execution {
            exit: Some(exit),
            stdout: stdout.into(),
            stderr: stderr.into(),
            timed_out: false,
        }
    }

    #[test]
    fn compiler_names_outside_the_allow_list_are_rejected() {
        for name in [
            "sh",
            "bash",
            "rustc",
            "cargo",
            "cc; rm -rf /",
            "gcc -O2",
            "./cc",
        ] {
            assert!(
                resolve_compiler(name, &toolchain()).is_err(),
                "accepted {name}"
            );
        }
    }

    #[test]
    fn compile_argv_uses_required_flags_and_runtime_sources() {
        let argv = compile_argv(
            Path::new("/usr/bin/gcc"),
            &toolchain(),
            Path::new("/repo"),
            &[PathBuf::from("/w/case.c")],
            Path::new("/w/case"),
        )
        .expect("argv");
        for flag in toolchain().required_flags {
            assert!(argv.contains(&flag));
        }
        assert!(argv
            .iter()
            .any(|item| item.ends_with("argorix_core_runtime.c")));
        assert!(!argv.iter().any(|item| item.ends_with(".h")));
        assert!(argv.contains(&"-o".to_string()));
    }

    #[test]
    fn forbidden_link_inputs_are_refused() {
        for item in [
            "/x/libstd.rlib",
            "/x/foo.rustlib",
            "/usr/bin/cargo",
            "rustc",
        ] {
            let argv = vec!["/usr/bin/gcc".to_string(), item.to_string()];
            assert!(check_argv(&argv, &toolchain()).is_err(), "accepted {item}");
        }
    }

    #[test]
    fn glob_matching_handles_the_declared_patterns() {
        assert!(glob_matches("*.rlib", "libstd.rlib"));
        assert!(!glob_matches("*.rlib", "libstd.rlibx"));
        assert!(glob_matches("cargo", "cargo"));
        assert!(!glob_matches("cargo", "cargo.exe"));
    }

    #[test]
    fn comparison_accepts_one_trailing_newline() {
        assert!(compare(&case(), &execution(0, "ARGORIX_RESULT:42\n", "")).is_empty());
        let trap = Case {
            expected_exit: 70,
            expected_stdout: String::new(),
            expected_stderr: "ARGORIX_TRAP:INTEGER_OVERFLOW".into(),
            ..case()
        };
        assert!(compare(&trap, &execution(70, "", "ARGORIX_TRAP:INTEGER_OVERFLOW\n")).is_empty());
    }

    #[test]
    fn comparison_detects_each_kind_of_mismatch() {
        assert_eq!(
            compare(&case(), &execution(1, "ARGORIX_RESULT:42\n", "")).len(),
            1
        );
        assert_eq!(
            compare(&case(), &execution(0, "ARGORIX_RESULT:41\n", "")).len(),
            1
        );
        assert_eq!(
            compare(&case(), &execution(0, "ARGORIX_RESULT:42\n", "noise\n")).len(),
            1
        );
        assert_eq!(
            compare(&case(), &execution(0, "ARGORIX_RESULT:42\n\n", "")).len(),
            1
        );
    }

    #[test]
    fn case_validation_rejects_traversal_and_odd_identifiers() {
        let base = Path::new(".");
        for file in [
            "../escape.argx",
            "/etc/passwd.argx",
            "..\\escape.argx",
            "sub/../../x.argx",
        ] {
            let candidate = Case {
                file: file.into(),
                ..case()
            };
            assert!(validate_case(&candidate, base).is_err(), "accepted {file}");
        }
        for id in ["Scalar", "a b", "x;y", ""] {
            let candidate = Case {
                id: id.into(),
                ..case()
            };
            assert!(validate_case(&candidate, base).is_err(), "accepted {id:?}");
        }
    }
}

#[cfg(test)]
mod sanitizer_tests {
    use super::*;

    #[test]
    fn leaks_undefined_behaviour_and_address_errors_are_findings() {
        let stderr = "\
==1219==ERROR: LeakSanitizer: detected memory leaks
Direct leak of 16 byte(s) in 1 object(s) allocated from:
case.c:12:5: runtime error: signed integer overflow
==42==ERROR: AddressSanitizer: heap-use-after-free on address 0x602
";
        let findings = sanitizer_findings(stderr);
        assert_eq!(findings.len(), 3);
        assert!(findings[0].contains("LeakSanitizer"));
        assert!(findings[1].contains("runtime error:"));
        assert!(findings[2].contains("AddressSanitizer:"));
    }

    #[test]
    fn ordinary_trap_output_is_not_a_finding() {
        assert!(sanitizer_findings("ARGORIX_TRAP:INTEGER_OVERFLOW\n").is_empty());
        assert!(sanitizer_findings("").is_empty());
    }

    #[test]
    fn the_sanitizer_flags_make_undefined_behaviour_fatal() {
        assert!(SANITIZER_FLAGS.contains(&"-fno-sanitize-recover=all"));
        assert!(SANITIZER_FLAGS
            .iter()
            .any(|flag| flag.contains("address") && flag.contains("undefined")));
    }

    #[test]
    fn sanitizer_flags_follow_the_declared_profile_rather_than_replacing_it() {
        let toolchain = Toolchain {
            allowed_compiler_names: vec!["gcc".into()],
            required_flags: vec!["-std=c11".into(), "-Werror".into()],
            runtime_sources: vec!["bootstrap/c/argorix_core_runtime.c".into()],
            forbidden_link_inputs: Vec::new(),
        };
        let argv = compile_argv_with(
            Path::new("/usr/bin/gcc"),
            &toolchain,
            Path::new("/repo"),
            &[PathBuf::from("/w/case.c")],
            Path::new("/w/case"),
            &SANITIZER_FLAGS,
        )
        .expect("argv");
        let werror = argv.iter().position(|item| item == "-Werror").unwrap();
        let sanitize = argv
            .iter()
            .position(|item| item.starts_with("-fsanitize="))
            .unwrap();
        assert!(
            werror < sanitize,
            "the declared flags stay in front: {argv:?}"
        );
    }
}
