"""Shared, oracle-free utilities for the adversarial evaluation harness.

Nothing in this module may read expected outcomes.  It only knows how to run
processes, hash bytes and recompute Argorix canonical digests with an
implementation that is independent of the Rust code under test.
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

REPO_ROOT = Path(__file__).resolve().parents[3]
EVAL_ROOT = Path(__file__).resolve().parents[1]

BINARIES = {
    "argorixc": "argorixc.exe" if os.name == "nt" else "argorixc",
    "argorix-vm": "argorix-vm.exe" if os.name == "nt" else "argorix-vm",
    "argorix-conformance": (
        "argorix-conformance.exe" if os.name == "nt" else "argorix-conformance"
    ),
}


class BinaryMissing(RuntimeError):
    """Raised when a production binary the campaign depends on is absent."""


# --------------------------------------------------------------------------
# canonical digests (independent reimplementation of crates/argorix_vm/evidence.rs)
# --------------------------------------------------------------------------
#
# The Rust side computes sha256 over `serde_json::to_vec(&value)`, i.e. the
# compact serialisation of the typed struct.  Artifacts on disk are written
# with `serde_json::to_string_pretty` of that same struct, so the on-disk key
# order is the struct declaration order.  Re-serialising the parsed document
# compactly therefore reproduces the hashed byte string without reusing any
# Rust code.


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def digest_json_file(path: Path) -> str | None:
    try:
        with path.open("r", encoding="utf-8") as handle:
            document = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return None
    return canonical_digest(document)


def digest_trace_ledger(trace_path: Path) -> str | None:
    try:
        with trace_path.open("r", encoding="utf-8") as handle:
            trace = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(trace, dict) or "events" not in trace:
        return None
    return canonical_digest(trace["events"])


def sha256_file(path: Path) -> str | None:
    try:
        data = path.read_bytes()
    except OSError:
        return None
    return "sha256:" + hashlib.sha256(data).hexdigest()


# --------------------------------------------------------------------------
# process execution
# --------------------------------------------------------------------------


class CommandResult(dict):
    """A recorded process invocation."""

    @property
    def exit_code(self) -> int | None:
        return self["exit_code"]

    @property
    def stdout(self) -> str:
        return self["stdout"]

    @property
    def stderr(self) -> str:
        return self["stderr"]

    @property
    def ok(self) -> bool:
        return self["exit_code"] == 0


def run_process(
    argv: Sequence[str],
    *,
    cwd: Path | None = None,
    env: Mapping[str, str] | None = None,
    timeout: float = 120.0,
    stdout_path: Path | None = None,
    input_text: str | None = None,
) -> CommandResult:
    """Run a real process and record everything observable about it."""

    started = time.perf_counter()
    timed_out = False
    exit_code: int | None
    stdout = ""
    stderr = ""
    process_env = dict(os.environ)
    if env:
        process_env.update(env)
    try:
        completed = subprocess.run(  # noqa: S603 - deliberate process execution
            list(argv),
            cwd=str(cwd) if cwd else None,
            env=process_env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            input=input_text,
        )
        exit_code = completed.returncode
        stdout = completed.stdout or ""
        stderr = completed.stderr or ""
    except subprocess.TimeoutExpired as expired:
        timed_out = True
        exit_code = None
        stdout = _as_text(expired.stdout)
        stderr = _as_text(expired.stderr)
    except FileNotFoundError as missing:
        raise BinaryMissing(str(missing)) from missing
    except OSError as error:  # pragma: no cover - platform dependent
        exit_code = None
        stderr = f"{type(error).__name__}: {error}"
    elapsed_ms = round((time.perf_counter() - started) * 1000.0, 3)

    if stdout_path is not None:
        stdout_path.parent.mkdir(parents=True, exist_ok=True)
        stdout_path.write_text(stdout, encoding="utf-8")

    return CommandResult(
        argv=list(argv),
        exit_code=exit_code,
        timed_out=timed_out,
        duration_ms=elapsed_ms,
        stdout=stdout,
        stderr=stderr,
        stdout_bytes=len(stdout.encode("utf-8")),
        stderr_bytes=len(stderr.encode("utf-8")),
    )


def _as_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return str(value)


# --------------------------------------------------------------------------
# environment manifest
# --------------------------------------------------------------------------


def resolve_binaries(bin_dir: Path) -> dict[str, Path]:
    resolved: dict[str, Path] = {}
    for key, filename in BINARIES.items():
        candidate = bin_dir / filename
        if not candidate.is_file():
            raise BinaryMissing(
                f"required production binary `{key}` is missing at {candidate}"
            )
        resolved[key] = candidate
    return resolved


def git_commit() -> str | None:
    result = run_process(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, timeout=30)
    if result.ok:
        return result.stdout.strip()
    return None


def git_dirty() -> bool | None:
    result = run_process(["git", "status", "--porcelain"], cwd=REPO_ROOT, timeout=60)
    if result.ok:
        return bool(result.stdout.strip())
    return None


def toolchain_versions(binaries: Mapping[str, Path]) -> dict[str, Any]:
    versions: dict[str, Any] = {}
    for name, path in binaries.items():
        result = run_process([str(path), "--version"], timeout=30)
        versions[name] = {
            "path": str(path),
            "sha256": sha256_file(path),
            "size_bytes": path.stat().st_size,
            "version": result.stdout.strip() or None,
        }
    for tool in ("rustc", "cargo"):
        candidate = _find_tool(tool)
        if candidate is None:
            versions[tool] = None
            continue
        result = run_process([candidate, "--version"], timeout=60)
        versions[tool] = result.stdout.strip() or None
    return versions


def _find_tool(name: str) -> str | None:
    from shutil import which

    found = which(name)
    if found:
        return found
    cargo_bin = Path.home() / ".cargo" / "bin" / (
        f"{name}.exe" if os.name == "nt" else name
    )
    return str(cargo_bin) if cargo_bin.is_file() else None


def environment_manifest(binaries: Mapping[str, Path]) -> dict[str, Any]:
    return {
        "commit": git_commit(),
        "worktree_dirty": git_dirty(),
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "version": platform.version(),
            "machine": platform.machine(),
            "python": sys.version.split()[0],
        },
        "toolchain": toolchain_versions(binaries),
    }


# --------------------------------------------------------------------------
# stable diagnostic classes
# --------------------------------------------------------------------------
#
# Diagnostics are compared by class, never by exact message text.  Order
# matters: the first pattern that matches wins.

DIAGNOSTIC_CLASSES: tuple[tuple[str, str], ...] = (
    ("unknown_policy_rule", r"unknown (policy )?rule"),
    ("unsupported_provider", r"unsupported (model|tool) provider"),
    ("provider_boundary", r"provider boundary|external provider .*(blocked|cannot execute)"),
    ("bytecode_verification_failed", r"bytecode verification failed"),
    ("policy_block_activated", r"activated block action"),
    ("capability_missing", r"without capability"),
    ("tool_not_declared", r"without declaring it in `tools`"),
    ("model_not_declared", r"without declaring it in `models`"),
    ("no_handler", r"no handler for message"),
    ("unknown_agent", r"unknown agent|unknown target agent|mailbox for internal agent .* does not exist"),
    ("invalid_injection_route", r"invalid injection|injection route|expected `from:to:act:type`"),
    ("runtime_profile_rejected", r"runtime profile .* rejected request"),
    ("unknown_runtime_profile", r"unknown runtime_execution_profile"),
    ("allowlist_rejected", r"allowlist|allowed_targets|allowed_capabilities"),
    ("evidence_verification_failed", r"evidence verification failed"),
    ("artifact_outside_portable_tree", r"outside the bundle portable tree"),
    ("artifact_unreadable", r"failed to (read|access)|No such file|cannot find the (file|path)"),
    ("invalid_json", r"invalid .*JSON|expected value|EOF while parsing|trailing characters"),
    ("semantic_error", r"^[^\n]*:\d+:\d+: error:"),
    ("missing_argument", r"required arguments were not provided|requires `--"),
    ("step_exhaustion", r"step (limit|budget) exhausted|max_steps"),
    ("harness_deadline", r"^__harness_deadline__$"),
)


def classify_diagnostic(text: str) -> str | None:
    if not text or not text.strip():
        return None
    for label, pattern in DIAGNOSTIC_CLASSES:
        if re.search(pattern, text, flags=re.IGNORECASE | re.MULTILINE):
            return label
    return "other"


def classify_all(*texts: str) -> list[str]:
    seen: list[str] = []
    for text in texts:
        label = classify_diagnostic(text)
        if label and label not in seen:
            seen.append(label)
    return seen


# --------------------------------------------------------------------------
# small helpers
# --------------------------------------------------------------------------


def load_json(path: Path) -> Any | None:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError):
        return None


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def file_size(path: Path) -> int | None:
    try:
        return path.stat().st_size
    except OSError:
        return None


def flatten(values: Iterable[Iterable[Any]]) -> list[Any]:
    return [item for group in values for item in group]
