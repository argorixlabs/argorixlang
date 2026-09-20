#!/usr/bin/env python3
"""Execute Argorix Core runtime cases through the transitional C backend.

ESP-008.R. The case manifest (tests/selfhost/runtime/cases.json) is the oracle;
`argorixc core-emit-c`, the C compiler, and the C1 runtime are implementations
under test. For every case the runner emits C, compiles it with an allow-listed
compiler through an argument array (never a shell), executes the program,
compares stdout, stderr, and exit status, and inspects the Linux executable for
Rust libraries, Rust symbols, and process-spawning imports.

Emission and execution are separate phases so that execution can be shown on a
host with no Rust toolchain:

    run.py emit --argorixc PATH --bundle DIR     # host with argorixc
    run.py run  --bundle DIR --cc gcc            # any Linux host
    run.py all  --argorixc PATH --cc gcc         # both, as CI does

This is transitional bootstrap tooling. Passing it shows that Core programs
execute as C without a Rust runtime; it does not show self-hosting, a native
backend, or independence of the whole toolchain from Rust.
"""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import hashlib
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_CASES = ROOT / "tests" / "selfhost" / "runtime" / "cases.json"
TOOLCHAIN = ROOT / "bootstrap" / "c" / "toolchain.json"
POLICY = ROOT / "conformance" / "core_c" / "policy.json"
DEFAULT_WORK = ROOT / "target" / "core-c"
GAPS = ROOT / "conformance" / "core_c" / "gaps" / "gaps.json"

REPORT_SCHEMA = 1
BUNDLE_SCHEMA = 1
CASE_ID = re.compile(r"^[a-z0-9_]+$")
COMPILER_NAME = re.compile(r"^[a-z0-9+_-]+$")
ELF_MAGIC = b"\x7fELF"
SCOPE = (
    "Transitional C execution of Core 0.1 runtime cases (ESP-008). "
    "Not self-hosting, not a native backend, not independence of the toolchain from Rust."
)


class RunnerError(Exception):
    """A configuration or environment problem that prevents a valid run."""


# --------------------------------------------------------------------------
# Small helpers


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise RunnerError(f"missing file: {path}") from error
    except json.JSONDecodeError as error:
        raise RunnerError(f"invalid JSON in {path}: {error}") from error


def display(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def decode(data: bytes) -> str:
    return data.decode("utf-8", errors="replace").replace("\r\n", "\n")


def strip_one_newline(text: str) -> str:
    return text[:-1] if text.endswith("\n") else text


def run_argv(argv: list[str], **kwargs: Any) -> subprocess.CompletedProcess[bytes]:
    """Run a command from an argument list. There is no shell=True path."""
    if not argv or not all(isinstance(item, str) for item in argv):
        raise RunnerError(f"invalid argument vector: {argv!r}")
    return subprocess.run(argv, shell=False, check=False, capture_output=True, **kwargs)


def first_line(argv: list[str]) -> str:
    try:
        completed = run_argv(argv, timeout=30)
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"unavailable: {error}"
    text = decode(completed.stdout or completed.stderr).strip()
    return text.splitlines()[0] if text else ""


def git_commit() -> str | None:
    try:
        completed = run_argv(["git", "-C", str(ROOT), "rev-parse", "HEAD"], timeout=30)
    except OSError:
        return None
    return decode(completed.stdout).strip() or None


def host_info() -> dict[str, Any]:
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        # Enforced only with --require-rust-free-host. The emit host needs Rust
        # to build argorixc; the execution host should not.
        "rust_tools_on_path": {
            name: shutil.which(name) for name in ("rustc", "cargo", "rustup")
        },
    }


# --------------------------------------------------------------------------
# Case manifest


def load_cases(path: pathlib.Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    manifest = load_json(path)
    if manifest.get("schema_version") != 1:
        raise RunnerError(f"unsupported cases schema_version in {path}")
    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        raise RunnerError(f"{path} declares no cases")
    seen: set[str] = set()
    for case in cases:
        validate_case(case, path.parent)
        if case["id"] in seen:
            raise RunnerError(f"duplicate case id: {case['id']}")
        seen.add(case["id"])
    return manifest, cases


def validate_case(case: Any, base: pathlib.Path) -> None:
    if not isinstance(case, dict):
        raise RunnerError(f"case is not an object: {case!r}")
    required = {
        "id": str,
        "file": str,
        "expected_exit": int,
        "expected_stdout": str,
        "expected_stderr": str,
    }
    for key, kind in required.items():
        if not isinstance(case.get(key), kind) or isinstance(case.get(key), bool):
            raise RunnerError(f"case {case.get('id')!r}: field {key!r} must be {kind.__name__}")
    if not CASE_ID.match(case["id"]):
        raise RunnerError(f"case id {case['id']!r} must match {CASE_ID.pattern}")
    relative = pathlib.PurePosixPath(case["file"])
    if relative.is_absolute() or ".." in relative.parts or "\\" in case["file"]:
        raise RunnerError(f"case {case['id']}: file must be a plain relative path")
    if not relative.name.endswith(".argx"):
        raise RunnerError(f"case {case['id']}: file must be a .argx source")
    if not (base / relative).is_file():
        raise RunnerError(f"case {case['id']}: missing source {base / relative}")


# --------------------------------------------------------------------------
# Emission (needs argorixc)


def emitter_available(argorixc: pathlib.Path) -> bool:
    try:
        completed = run_argv([str(argorixc), "--help"], timeout=60)
    except OSError as error:
        raise RunnerError(f"cannot execute argorixc at {argorixc}: {error}") from error
    return "core-emit-c" in decode(completed.stdout + completed.stderr)


def emit_once(argorixc: pathlib.Path, source: pathlib.Path, output: pathlib.Path) -> tuple[int, str]:
    completed = run_argv(
        [str(argorixc), "core-emit-c", str(source), "--output", str(output)],
        cwd=str(ROOT),
        timeout=120,
    )
    return completed.returncode, decode(completed.stderr).strip()


def emit(argorixc: pathlib.Path, cases_path: pathlib.Path, bundle: pathlib.Path) -> dict[str, Any]:
    argorixc = argorixc.resolve()
    if not argorixc.is_file():
        raise RunnerError(f"argorixc not found: {argorixc}")
    if not emitter_available(argorixc):
        raise RunnerError(
            "argorixc has no `core-emit-c` command; the ESP-008 emitter is not in this build"
        )
    _, cases = load_cases(cases_path)
    bundle.mkdir(parents=True, exist_ok=True)
    entries = []
    for case in cases:
        source = cases_path.parent / case["file"]
        first = bundle / f"{case['id']}.c"
        second = bundle / f"{case['id']}.c.repeat"
        code, stderr = emit_once(argorixc, source, first)
        entry: dict[str, Any] = {
            "id": case["id"],
            "source": display(source),
            "source_sha256": sha256_file(source),
            "emit_exit": code,
            "emit_stderr": stderr,
        }
        if code == 0 and first.is_file():
            repeat_code, _ = emit_once(argorixc, source, second)
            entry["c_file"] = first.name
            entry["c_sha256"] = sha256_file(first)
            entry["deterministic"] = repeat_code == 0 and second.is_file() and (
                sha256_file(second) == entry["c_sha256"]
            )
            second.unlink(missing_ok=True)
        else:
            entry["deterministic"] = False
        entries.append(entry)
    manifest = {
        "schema_version": BUNDLE_SCHEMA,
        "kind": "argorix-core-c-emit-bundle",
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "commit": git_commit(),
        "host": host_info(),
        "argorixc": {
            "path": display(argorixc),
            "sha256": sha256_file(argorixc),
            "version": first_line([str(argorixc), "--version"]),
        },
        "cases_manifest": display(cases_path),
        "cases_manifest_sha256": sha256_file(cases_path),
        "cases": entries,
    }
    (bundle / "bundle.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


# --------------------------------------------------------------------------
# Compilation (allow-listed compiler, argument arrays only)


def resolve_compiler(requested: str, toolchain: dict[str, Any]) -> pathlib.Path:
    allowed = toolchain.get("allowed_compiler_names", [])
    candidate = pathlib.Path(requested)
    if candidate.is_absolute():
        # Explicitly configured absolute path, as bootstrap/c/toolchain.json allows.
        if not candidate.is_file() or not os.access(candidate, os.X_OK):
            raise RunnerError(f"compiler is not an executable file: {candidate}")
        if candidate.stem.lower() in {"rustc", "cargo"}:
            raise RunnerError(f"compiler path names a Rust tool: {candidate}")
        return candidate
    if not COMPILER_NAME.match(requested) or requested not in allowed:
        raise RunnerError(
            f"compiler {requested!r} is not allow-listed; use one of {allowed} or an absolute path"
        )
    found = shutil.which(requested)
    if found is None:
        raise RunnerError(f"compiler {requested!r} is not on PATH")
    return pathlib.Path(found)


def check_argv(argv: list[str], toolchain: dict[str, Any]) -> None:
    forbidden = toolchain.get("forbidden_link_inputs", [])
    for item in argv:
        name = pathlib.PurePath(item).name
        for pattern in forbidden:
            if fnmatch.fnmatch(name, pattern) or fnmatch.fnmatch(pathlib.PurePath(name).stem, pattern):
                raise RunnerError(f"compile argument {item!r} matches forbidden input {pattern!r}")


def compile_argv(
    compiler: pathlib.Path,
    toolchain: dict[str, Any],
    sources: list[pathlib.Path],
    output: pathlib.Path,
    extra: list[str] | None = None,
) -> list[str]:
    flags = list(toolchain.get("required_flags", []))
    if not flags:
        raise RunnerError("toolchain.json declares no required_flags")
    runtime_c = [ROOT / item for item in toolchain.get("runtime_sources", []) if item.endswith(".c")]
    include = sorted({str((ROOT / item).parent) for item in toolchain.get("runtime_sources", [])})
    argv = [str(compiler), *flags]
    for directory in include:
        argv += ["-I", directory]
    argv += [str(path) for path in sources]
    argv += [str(path) for path in runtime_c]
    argv += ["-o", str(output)]
    argv += extra or []
    check_argv(argv, toolchain)
    return argv


# --------------------------------------------------------------------------
# Dependency inspection (Linux ELF via readelf)


def parse_needed(dynamic_section: str) -> list[str]:
    return re.findall(r"\(NEEDED\)\s+Shared library:\s+\[([^\]]+)\]", dynamic_section)


def parse_symbols(symbol_table: str) -> list[tuple[str, str]]:
    """Return (name, section index) pairs from `readelf -W -s` output."""
    symbols = []
    for line in symbol_table.splitlines():
        fields = line.split()
        # Num: Value Size Type Bind Vis Ndx Name
        if len(fields) >= 8 and fields[0].endswith(":") and fields[0][:-1].isdigit():
            name = fields[7].split("@")[0]
            if name:
                symbols.append((name, fields[6]))
    return symbols


def classify(
    needed: list[str],
    symbols: list[tuple[str, str]],
    content: bytes,
    policy: dict[str, Any],
) -> list[str]:
    """Return every policy violation; an empty list means the binary is clean."""
    violations = []
    allowed = set(policy["allowed_needed_libraries"])
    for library in needed:
        if library not in allowed:
            violations.append(f"NEEDED library not allow-listed: {library}")
        for pattern in policy["forbidden_library_patterns"]:
            if re.search(pattern, library):
                violations.append(f"NEEDED library matches {pattern!r}: {library}")
    names = sorted({name for name, _ in symbols})
    for name in names:
        for pattern in policy["forbidden_symbol_patterns"]:
            if re.search(pattern, name):
                violations.append(f"symbol matches {pattern!r}: {name}")
    imported = sorted({name for name, index in symbols if index == "UND"})
    for name in imported:
        if name in set(policy["forbidden_imports"]):
            violations.append(f"imports process/loader function: {name}")
    for marker in policy["forbidden_byte_markers"]:
        if marker.encode("utf-8") in content:
            violations.append(f"contains byte marker: {marker}")
    return violations


def inspect_binary(executable: pathlib.Path, policy: dict[str, Any]) -> dict[str, Any]:
    content = executable.read_bytes()
    if not content.startswith(ELF_MAGIC):
        return {
            "format": "not-elf",
            "passed": False,
            "violations": ["dependency inspection supports Linux ELF only"],
        }
    readelf = shutil.which("readelf")
    if readelf is None:
        raise RunnerError("readelf is required for dependency inspection")
    dynamic = decode(run_argv([readelf, "-W", "-d", str(executable)], timeout=60).stdout)
    table = decode(run_argv([readelf, "-W", "-s", str(executable)], timeout=60).stdout)
    needed = parse_needed(dynamic)
    symbols = parse_symbols(table)
    violations = classify(needed, symbols, content, policy)
    return {
        "format": "elf",
        "needed": needed,
        "imported_symbols": sorted({name for name, index in symbols if index == "UND"}),
        "symbol_count": len(symbols),
        "violations": violations,
        "passed": not violations,
    }


# --------------------------------------------------------------------------
# Execution and comparison


def compare(case: dict[str, Any], exit_code: int | None, stdout: str, stderr: str) -> list[str]:
    mismatches = []
    if exit_code != case["expected_exit"]:
        mismatches.append(f"exit {exit_code} != expected {case['expected_exit']}")
    if strip_one_newline(stdout) != case["expected_stdout"]:
        mismatches.append(f"stdout {stdout!r} != expected {case['expected_stdout']!r}")
    if strip_one_newline(stderr) != case["expected_stderr"]:
        mismatches.append(f"stderr {stderr!r} != expected {case['expected_stderr']!r}")
    return mismatches


def stack_limiter(mib):
    """Pin the child's stack so a stack-overflow gap reproduces the same way."""
    if mib is None or os.name != "posix":
        return None
    import resource  # POSIX only

    def apply() -> None:  # pragma: no cover - runs in the forked child
        _, hard = resource.getrlimit(resource.RLIMIT_STACK)
        resource.setrlimit(resource.RLIMIT_STACK, (mib * 1024 * 1024, hard))

    return apply


def execute(executable: pathlib.Path, timeout: int, stack_mib=None) -> dict[str, Any]:
    try:
        completed = run_argv([str(executable)], stdin=subprocess.DEVNULL, timeout=timeout,
                             preexec_fn=stack_limiter(stack_mib))
    except subprocess.TimeoutExpired:
        return {"exit": None, "stdout": "", "stderr": "", "timed_out": True}
    return {
        "exit": completed.returncode,
        "stdout": decode(completed.stdout),
        "stderr": decode(completed.stderr),
        "timed_out": False,
    }


def run_case(
    case: dict[str, Any],
    entry: dict[str, Any],
    bundle: pathlib.Path,
    work: pathlib.Path,
    compiler: pathlib.Path,
    toolchain: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    result: dict[str, Any] = {"id": case["id"], "expected": {
        "exit": case["expected_exit"],
        "stdout": case["expected_stdout"],
        "stderr": case["expected_stderr"],
    }}
    failures: list[str] = []
    if entry.get("emit_exit") != 0 or "c_file" not in entry:
        result.update(failures=[f"emission failed: {entry.get('emit_stderr', '')}"], passed=False)
        return result
    if not entry.get("deterministic"):
        failures.append("emission is not deterministic across two runs")
    c_file = bundle / entry["c_file"]
    if sha256_file(c_file) != entry["c_sha256"]:
        result.update(failures=["generated C does not match bundle hash"], passed=False)
        return result
    result["c_sha256"] = entry["c_sha256"]

    executable = work / case["id"]
    argv = compile_argv(compiler, toolchain, [c_file], executable)
    compiled = run_argv(argv, timeout=300)
    result["compile"] = {
        "argv": [display(pathlib.Path(item)) if os.sep in item or "/" in item else item for item in argv],
        "exit": compiled.returncode,
        "diagnostics": decode(compiled.stderr).strip(),
    }
    if compiled.returncode != 0 or not executable.is_file():
        failures.append("C compilation failed")
        result.update(failures=failures, passed=False)
        return result
    result["executable_sha256"] = sha256_file(executable)

    observed = execute(executable, policy["execution_timeout_seconds"])
    result["observed"] = observed
    if observed["timed_out"]:
        failures.append(f"timed out after {policy['execution_timeout_seconds']} s")
    else:
        failures += compare(case, observed["exit"], observed["stdout"], observed["stderr"])

    inspection = inspect_binary(executable, policy)
    result["dependencies"] = inspection
    failures += inspection["violations"]

    result.update(failures=failures, passed=not failures)
    return result


# --------------------------------------------------------------------------
# Negative controls: the checks must be able to fail


SPAWN_SENSOR = """#include <stdlib.h>
int main(void) { return system("true") == 0 ? 0 : 1; }
"""
SYMBOL_SENSOR = """void __rust_alloc(void);
void __rust_alloc(void) {}
int main(void) { __rust_alloc(); return 0; }
"""
LIBRARY_SENSOR = "int rust_sensor_value(void);\nint rust_sensor_value(void) { return 1; }\n"
LIBRARY_SENSOR_MAIN = "int rust_sensor_value(void);\nint main(void) { return rust_sensor_value(); }\n"


def sensor_binary(
    name: str,
    source_text: str,
    work: pathlib.Path,
    compiler: pathlib.Path,
    extra: list[str] | None = None,
) -> pathlib.Path:
    """Compile (never execute) a sensor program without the Argorix runtime."""
    source = work / f"{name}.c"
    source.write_text(source_text, encoding="utf-8")
    output = work / name
    completed = run_argv([str(compiler), str(source), "-o", str(output), *(extra or [])], timeout=300)
    if completed.returncode != 0:
        raise RunnerError(f"sensor {name} did not compile: {decode(completed.stderr)}")
    return output


def negative_controls(
    results: list[dict[str, Any]],
    cases: dict[str, dict[str, Any]],
    work: pathlib.Path,
    compiler: pathlib.Path,
    policy: dict[str, Any],
) -> list[dict[str, Any]]:
    controls = []
    executed = next((item for item in results if "observed" in item), None)
    if executed is None:
        controls.append({"id": "comparator_detects_mismatch", "detected": False,
                         "detail": "no executed case available"})
    else:
        case = dict(cases[executed["id"]])
        case["expected_stdout"] += "_MUTATED"
        case["expected_exit"] += 1
        observed = executed["observed"]
        mismatches = compare(case, observed["exit"], observed["stdout"], observed["stderr"])
        controls.append({"id": "comparator_detects_mismatch", "detected": len(mismatches) == 2,
                         "detail": mismatches})

    sensors = work / "sensors"
    sensors.mkdir(parents=True, exist_ok=True)
    spawn = inspect_binary(sensor_binary("spawn_sensor", SPAWN_SENSOR, sensors, compiler), policy)
    controls.append({"id": "inspector_detects_process_spawn",
                     "detected": any("system" in item for item in spawn["violations"]),
                     "detail": spawn["violations"]})
    symbol = inspect_binary(sensor_binary("symbol_sensor", SYMBOL_SENSOR, sensors, compiler), policy)
    controls.append({"id": "inspector_detects_rust_symbol",
                     "detected": any("__rust_alloc" in item for item in symbol["violations"]),
                     "detail": symbol["violations"]})
    library_source = sensors / "rust_sensor.c"
    library_source.write_text(LIBRARY_SENSOR, encoding="utf-8")
    shared = sensors / "librust_sensor.so"
    built = run_argv([str(compiler), "-shared", "-fPIC", str(library_source), "-o", str(shared)],
                     timeout=300)
    if built.returncode != 0:
        raise RunnerError(f"library sensor did not compile: {decode(built.stderr)}")
    linked = sensor_binary("library_sensor", LIBRARY_SENSOR_MAIN, sensors, compiler,
                           ["-L", str(sensors), "-lrust_sensor"])
    library = inspect_binary(linked, policy)
    controls.append({"id": "inspector_detects_rust_library",
                     "detected": any("librust_sensor.so" in item for item in library["violations"]),
                     "detail": library["violations"]})
    return controls


# --------------------------------------------------------------------------
# Known gaps (issue #27): record today's behaviour so a fix gets noticed


STILL_OPEN = "STILL_OPEN"
FIXED = "FIXED"
CHANGED = "CHANGED"


def classify_gap(gap: dict[str, Any], outcome: dict[str, Any]) -> tuple[str, str]:
    """Compare one gap's observed behaviour with what the manifest recorded.

    STILL_OPEN: the recorded defect is reproduced.
    FIXED:      the program now behaves as the spec requires; promote it.
    CHANGED:    something else happens, so the record is stale.
    """
    kind = gap["kind"]
    spec_stdout = gap["spec_expected_stdout"]
    signature = gap.get("signature", "")
    emit_code = outcome["emit_exit"]
    if kind == "emit_rejected" and emit_code != 0:
        if signature in outcome["emit_stderr"]:
            return STILL_OPEN, "emission still rejects it: " + signature
        return CHANGED, "emission fails with a different message: " + outcome["emit_stderr"][:160]
    if emit_code != 0:
        return CHANGED, "emission now fails: " + outcome["emit_stderr"][:160]
    if outcome["compile_exit"] is None:
        return CHANGED, "emission succeeded but nothing was compiled"
    if outcome["compile_exit"] != 0:
        if kind == "compile_error" and signature in outcome["compile_diagnostics"]:
            return STILL_OPEN, "C compilation still fails: " + signature
        return CHANGED, "C compilation fails: " + outcome["compile_diagnostics"][:160]

    observed = strip_one_newline(outcome["stdout"])
    matches_spec = observed == spec_stdout and outcome["exit"] == 0
    if kind == "crash":
        if matches_spec:
            return FIXED, "the program now runs to the expected result"
        if outcome["exit"] == 70 and outcome["stderr"].startswith("ARGORIX_TRAP:"):
            return FIXED, "now a typed trap: " + strip_one_newline(outcome["stderr"])
        if outcome["exit"] == gap.get("observed_exit"):
            return STILL_OPEN, "still exits %s instead of a typed trap" % outcome["exit"]
        return CHANGED, "exit %s, stdout %r, stderr %r" % (outcome["exit"], observed, outcome["stderr"][:120])
    if matches_spec:
        return FIXED, "the program now produces the result the spec requires"
    if kind == "wrong_result" and observed == gap.get("observed_stdout") \
            and outcome["exit"] == gap.get("observed_exit"):
        return STILL_OPEN, "still prints %r instead of %r" % (observed, spec_stdout)
    return CHANGED, "exit %s, stdout %r, stderr %r" % (outcome["exit"], observed, outcome["stderr"][:120])


def observe_gap(
    gap: dict[str, Any],
    argorixc: pathlib.Path,
    work: pathlib.Path,
    compiler: pathlib.Path,
    toolchain: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    source = GAPS.parent / gap["file"]
    generated = work / (gap["id"] + ".c")
    emit_code, emit_stderr = emit_once(argorixc, source, generated)
    outcome: dict[str, Any] = {
        "emit_exit": emit_code,
        "emit_stderr": emit_stderr,
        "compile_exit": None,
        "compile_diagnostics": "",
        "exit": None,
        "stdout": "",
        "stderr": "",
    }
    if emit_code != 0 or not generated.is_file():
        return outcome
    executable = work / gap["id"]
    compiled = run_argv(compile_argv(compiler, toolchain, [generated], executable), timeout=300)
    outcome["compile_exit"] = compiled.returncode
    outcome["compile_diagnostics"] = decode(compiled.stderr).strip()
    if compiled.returncode != 0 or not executable.is_file():
        return outcome
    outcome.update(execute(executable, policy["execution_timeout_seconds"], gap.get("stack_limit_mib")))
    return outcome


def check_gaps(argorixc: pathlib.Path, compiler_name: str, work: pathlib.Path) -> dict[str, Any]:
    toolchain = load_json(TOOLCHAIN)
    policy = load_json(POLICY)
    manifest = load_json(GAPS)
    if manifest.get("schema_version") != 1:
        raise RunnerError("unsupported gaps schema_version in %s" % GAPS)
    argorixc = argorixc.resolve()
    if not argorixc.is_file():
        raise RunnerError("argorixc not found: %s" % argorixc)
    if not emitter_available(argorixc):
        raise RunnerError("argorixc has no `core-emit-c` command")
    compiler = resolve_compiler(compiler_name, toolchain)
    work.mkdir(parents=True, exist_ok=True)
    results = []
    for gap in manifest["gaps"]:
        outcome = observe_gap(gap, argorixc, work, compiler, toolchain, policy)
        status, detail = classify_gap(gap, outcome)
        results.append({
            "id": gap["id"], "kind": gap["kind"], "issue": gap.get("issue"),
            "status": status, "detail": detail, "observed": outcome,
        })
    counts = {status: sum(1 for item in results if item["status"] == status)
              for status in (STILL_OPEN, FIXED, CHANGED)}
    return {
        "schema_version": REPORT_SCHEMA,
        "task": "ESP-008.R gap corpus",
        "scope": "Known defects of the transitional C backend (issue #27). A gap that starts "
                 "passing must be promoted into tests/selfhost/runtime/cases.json.",
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "commit": git_commit(),
        "recorded_commit": manifest.get("recorded_commit"),
        "host": host_info(),
        "compiler": {"requested": compiler_name, "path": str(compiler),
                     "version": first_line([str(compiler), "--version"])},
        "argorixc": {"path": display(argorixc), "sha256": sha256_file(argorixc),
                     "version": first_line([str(argorixc), "--version"])},
        "gaps_total": len(results),
        "counts": counts,
        "gaps": results,
        # A fixed gap is good news, not a build failure. A different failure means
        # the record is stale and someone has to look at it.
        "overall_pass": counts[CHANGED] == 0,
    }


def print_gap_summary(report: dict[str, Any], annotate: bool) -> None:
    for item in report["gaps"]:
        print("%-10s %s: %s" % (item["status"], item["id"], item["detail"]))
        if annotate and item["status"] != STILL_OPEN:
            title = "Core C gap fixed" if item["status"] == FIXED else "Core C gap changed"
            print("::notice title=%s::%s: %s" % (title, item["id"], item["detail"]))
    counts = report["counts"]
    print("gaps %d: %d still open, %d fixed, %d changed; recorded at %s; overall_pass=%s"
          % (report["gaps_total"], counts[STILL_OPEN], counts[FIXED], counts[CHANGED],
             report["recorded_commit"], report["overall_pass"]))
    if counts[FIXED]:
        print("Promote each fixed gap into tests/selfhost/runtime/cases.json (Codex lane) "
              "and drop it from conformance/core_c/gaps/gaps.json.")


# --------------------------------------------------------------------------
# Run phase and report


def load_bundle(bundle: pathlib.Path, cases_path: pathlib.Path) -> dict[str, Any]:
    manifest = load_json(bundle / "bundle.json")
    if manifest.get("schema_version") != BUNDLE_SCHEMA or manifest.get("kind") != "argorix-core-c-emit-bundle":
        raise RunnerError(f"{bundle} is not an emit bundle")
    if manifest.get("cases_manifest_sha256") != sha256_file(cases_path):
        raise RunnerError("bundle was emitted from a different cases.json than the one given")
    return manifest


def run(
    bundle: pathlib.Path,
    cases_path: pathlib.Path,
    compiler_name: str,
    work: pathlib.Path,
    with_controls: bool,
    require_rust_free: bool = False,
) -> dict[str, Any]:
    host = host_info()
    present = sorted(name for name, path in host["rust_tools_on_path"].items() if path)
    if require_rust_free and present:
        raise RunnerError(f"--require-rust-free-host, but found on PATH: {', '.join(present)}")
    toolchain = load_json(TOOLCHAIN)
    policy = load_json(POLICY)
    manifest = load_bundle(bundle, cases_path)
    _, cases = load_cases(cases_path)
    by_id = {case["id"]: case for case in cases}
    entries = {entry["id"]: entry for entry in manifest["cases"]}
    if set(entries) != set(by_id):
        raise RunnerError("bundle cases do not match the cases manifest")
    compiler = resolve_compiler(compiler_name, toolchain)
    work.mkdir(parents=True, exist_ok=True)

    results = [
        run_case(case, entries[case["id"]], bundle, work, compiler, toolchain, policy)
        for case in cases
    ]
    controls = negative_controls(results, by_id, work, compiler, policy) if with_controls else []
    cases_passed = sum(1 for item in results if item["passed"])
    controls_ok = all(item["detected"] for item in controls)
    return {
        "schema_version": REPORT_SCHEMA,
        "task": "ESP-008.R",
        "scope": SCOPE,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "commit": git_commit(),
        "execution_host": host,
        "rust_free_host_required": require_rust_free,
        "emission": {key: manifest[key] for key in ("commit", "host", "argorixc", "created_utc")},
        "compiler": {
            "requested": compiler_name,
            "path": str(compiler),
            "version": first_line([str(compiler), "--version"]),
        },
        "inputs_sha256": {
            display(pathlib.Path(__file__)): sha256_file(pathlib.Path(__file__)),
            display(cases_path): sha256_file(cases_path),
            display(TOOLCHAIN): sha256_file(TOOLCHAIN),
            display(POLICY): sha256_file(POLICY),
            **{item: sha256_file(ROOT / item) for item in toolchain.get("runtime_sources", [])},
        },
        "cases_total": len(results),
        "cases_passed": cases_passed,
        "cases": results,
        "negative_controls": controls,
        "negative_controls_run": with_controls,
        "overall_pass": cases_passed == len(results) and with_controls and controls_ok,
    }


def print_summary(report: dict[str, Any]) -> None:
    for item in report["cases"]:
        status = "PASS" if item["passed"] else "FAIL"
        print(f"{status} {item['id']}")
        for failure in item.get("failures", []):
            print(f"     {failure}")
    for control in report["negative_controls"]:
        status = "DETECTED" if control["detected"] else "MISSED"
        print(f"{status} negative control {control['id']}")
    print(
        f"cases {report['cases_passed']}/{report['cases_total']}; "
        f"compiler {report['compiler']['version']}; overall_pass={report['overall_pass']}"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    def common(command: argparse.ArgumentParser) -> None:
        command.add_argument("--cases", type=pathlib.Path, default=DEFAULT_CASES)
        command.add_argument("--work-dir", type=pathlib.Path, default=DEFAULT_WORK)

    emit_cmd = sub.add_parser("emit", help="emit C for every case into a bundle")
    emit_cmd.add_argument("--argorixc", type=pathlib.Path, required=True)
    emit_cmd.add_argument("--bundle", type=pathlib.Path)
    common(emit_cmd)

    for name, help_text in (("run", "compile, execute, and inspect a bundle"),
                            ("all", "emit, then run, in one process")):
        command = sub.add_parser(name, help=help_text)
        if name == "all":
            command.add_argument("--argorixc", type=pathlib.Path, required=True)
        command.add_argument("--bundle", type=pathlib.Path)
        command.add_argument("--cc", required=True, help="allow-listed compiler name or absolute path")
        command.add_argument("--report", type=pathlib.Path)
        command.add_argument("--skip-negative-controls", action="store_true",
                             help="run cases only; the report then cannot pass")
        command.add_argument("--require-rust-free-host", action="store_true",
                             help="fail if rustc, cargo, or rustup is on PATH of the execution host")
        common(command)

    gaps_cmd = sub.add_parser("gaps", help="check the known-gap corpus (issue #27)")
    gaps_cmd.add_argument("--argorixc", type=pathlib.Path, required=True)
    gaps_cmd.add_argument("--cc", required=True, help="allow-listed compiler name or absolute path")
    gaps_cmd.add_argument("--report", type=pathlib.Path)
    gaps_cmd.add_argument("--github-annotations", action="store_true",
                          help="print ::notice lines for gaps that are fixed or changed")
    gaps_cmd.add_argument("--work-dir", type=pathlib.Path, default=DEFAULT_WORK)

    args = parser.parse_args(argv)
    if args.command == "gaps":
        work = args.work_dir.resolve()
        try:
            report = check_gaps(args.argorixc, args.cc, work / "gaps")
        except RunnerError as error:
            print("error: %s" % error, file=sys.stderr)
            return 2
        report_path = (args.report or work / "gaps-report.json").resolve()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print_gap_summary(report, args.github_annotations)
        print("report: %s" % report_path)
        return 0 if report["overall_pass"] else 1

    cases_path = args.cases.resolve()
    work = args.work_dir.resolve()
    bundle = (args.bundle or work / "bundle").resolve()
    try:
        if args.command in ("emit", "all"):
            manifest = emit(args.argorixc, cases_path, bundle)
            emitted = sum(1 for item in manifest["cases"] if item["emit_exit"] == 0)
            print(f"emitted {emitted}/{len(manifest['cases'])} cases into {bundle}")
            if args.command == "emit":
                return 0 if emitted == len(manifest["cases"]) else 1
        report = run(bundle, cases_path, args.cc, work / "build",
                     not args.skip_negative_controls, args.require_rust_free_host)
    except RunnerError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    report_path = (args.report or work / "report.json").resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print_summary(report)
    print(f"report: {report_path}")
    return 0 if report["overall_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
