#!/usr/bin/env python3
"""Reproducibly inventory Argorix chatbot runtime artifacts."""

import argparse
import csv
import json
import re
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from atomic_io import atomic_publish, atomic_write_text


ARTIFACT_NAMES = (
    "session.argx",
    "session.argbc.json",
    "session.trace.json",
    "session.security.json",
    "session.evidence.json",
)
JSON_ARTIFACTS = ARTIFACT_NAMES[1:]
DIGEST_PATTERN = re.compile(r"^sha256:[0-9a-fA-F]{64}$")
REPARSE_POINT = 0x400
SECRET_ASSIGNMENT = re.compile(
    r"(?i)\b(api[_ -]?key|password|passwd|token|secret)"
    r"(\s*[:=]\s*)[\"']?[^\s,\"';]+"
)
BEARER_SECRET = re.compile(r"(?i)\bBearer\s+[^\s,;]+")
KEY_SHAPE = re.compile(r"\b(?:sk|rk|pk|tok)[_-][A-Za-z0-9_-]{8,}\b")


def _mapping(value):
    return value if isinstance(value, dict) else {}


def _list(value):
    return value if isinstance(value, list) else []


def _integer(value):
    return value if type(value) is int else None


def _sorted_strings(value):
    return sorted(str(item) for item in _list(value))


def _sanitize_text(value, limit=200):
    if not isinstance(value, str):
        return None
    text = value.replace("\x00", "")
    text = BEARER_SECRET.sub("Bearer [REDACTED]", text)
    text = SECRET_ASSIGNMENT.sub(
        lambda match: f"{match.group(1)}{match.group(2)}[REDACTED]", text
    )
    text = KEY_SHAPE.sub("[REDACTED]", text)
    if len(text) > limit:
        text = text[:limit] + "…"
    return text


def _sanitize_strings(value, limit=120):
    return sorted(
        sanitized
        for item in _list(value)
        if (sanitized := _sanitize_text(item, limit)) is not None
    )


def _names(value):
    value = _mapping(value)
    if isinstance(value.get("names"), list):
        return _sanitize_strings(value["names"])
    return sorted(
        sanitized
        for item in _list(value)
        if isinstance(item, dict) and "name" in item
        if (sanitized := _sanitize_text(item["name"], 120)) is not None
    )


def _is_link_or_reparse(path):
    try:
        if path.is_symlink():
            return True
        is_junction = getattr(path, "is_junction", None)
        if is_junction is not None and is_junction():
            return True
        attributes = getattr(path.lstat(), "st_file_attributes", 0)
        return bool(attributes & REPARSE_POINT)
    except OSError:
        return True


def _contained_resolved_path(root, path):
    if _is_link_or_reparse(path):
        return None
    try:
        resolved = path.resolve(strict=True)
    except OSError:
        return None
    return resolved if resolved.is_relative_to(root) else None


def _load_json(path, errors):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        errors[path.name] = f"{type(exc).__name__}: {exc}"
        return {}


def _normalize_provider_boundary(value):
    boundary = _mapping(value)
    contracts = [
        item for item in _list(boundary.get("declarative_contracts"))
        if isinstance(item, dict)
    ]
    return {
        "executable_providers": _sanitize_strings(
            boundary.get("executable_providers")
        ),
        "declarative_contract_names": sorted(
            name
            for item in contracts
            if (name := _sanitize_text(item.get("name"), 120)) is not None
        ),
        "external_contracts_total": _integer(
            boundary.get("external_contracts_total")
        ),
        "external_execution_blocked": (
            boundary.get("external_execution_blocked")
            if isinstance(boundary.get("external_execution_blocked"), bool)
            else None
        ),
        "blocked_attempts": _integer(boundary.get("blocked_attempts")),
    }


def _normalize_session(root, directory, publish_prompt=False):
    artifacts = {}
    for name in ARTIFACT_NAMES:
        path = directory / name
        safe_path = _contained_resolved_path(root, path) if path.exists() else None
        present = safe_path is not None and safe_path.is_file()
        artifacts[name] = {
            "present": present,
            "size_bytes": safe_path.stat().st_size if present else 0,
        }

    errors = {}
    documents = {
        name: _load_json(_contained_resolved_path(root, directory / name), errors)
        for name in JSON_ARTIFACTS
        if artifacts[name]["present"]
    }
    bytecode = _mapping(documents.get("session.argbc.json"))
    raw_trace = documents.get("session.trace.json")
    trace = _mapping(raw_trace)
    security = _mapping(documents.get("session.security.json"))
    evidence = _mapping(documents.get("session.evidence.json"))
    policy = _mapping(security.get("policy") or trace.get("policy_report"))
    violations = [item for item in _list(policy.get("violations")) if isinstance(item, dict)]
    ledger = _mapping(security.get("ledger"))
    event_kinds = _mapping(ledger.get("event_kinds"))
    if not event_kinds:
        event_kinds = {}
        for event in _list(trace.get("events")):
            if isinstance(event, dict):
                kind = event.get("event_type") or event.get("kind")
                if kind is not None:
                    event_kinds[str(kind)] = event_kinds.get(str(kind), 0) + 1
    event_kinds = {
        key: event_kinds[key]
        for key in sorted(event_kinds)
        if type(event_kinds[key]) is int
    }
    passports = _mapping(security.get("agent_passports"))
    boundary = _normalize_provider_boundary(security.get("provider_boundary"))
    digest_values = {}
    for key in ("bytecode_digest", "trace_digest", "report_digest", "ledger_digest"):
        value = evidence.get(key)
        digest_values[key] = {
            "value": value,
            "valid": isinstance(value, str) and DIGEST_PATTERN.fullmatch(value) is not None,
        }

    # Current traces describe the injected UserPrompt under ``injected``.
    # Production artifacts redact its value, so absence remains explicit. If a
    # trace producer includes the authorized content, read only that bounded
    # field; never recursively search payloads where secret values also live.
    injected = _mapping(trace.get("injected"))
    trace_assessable = (
        artifacts["session.trace.json"]["present"]
        and "session.trace.json" not in errors
        and isinstance(raw_trace, dict)
    )
    prompt_content_present = (
        isinstance(injected.get("content"), str) if trace_assessable else None
    )
    prompt = (
        _sanitize_text(injected.get("content"), 2000)
        if publish_prompt
        else None
    )

    execution = _mapping(security.get("execution"))
    security_checks = trace.get("security_checks")
    if security_checks is None:
        security_checks = security.get("security_checks")
    return {
        "request_id": directory.name,
        "complete": all(item["present"] for item in artifacts.values()),
        "artifact_count": sum(item["present"] for item in artifacts.values()),
        "artifacts": artifacts,
        "json_errors": errors,
        "execution_status": execution.get("status", trace.get("status")),
        "policy_passed": policy.get("passed"),
        "review_required": policy.get("review_required"),
        "policy_violation_count": len(violations),
        "policy_violation_rules": sorted(
            text for item in violations
            if (text := _sanitize_text(item.get("rule"), 200)) is not None
        ),
        "policy_violation_reasons": sorted(
            {
                text for item in violations
                if (text := _sanitize_text(item.get("reason"), 500)) is not None
            }
        ),
        "security_checks": security_checks,
        "ledger_events_total": (
            _integer(ledger.get("events_total"))
            if "events_total" in ledger
            else sum(event_kinds.values())
        ),
        "ledger_event_kinds": event_kinds,
        "passport_total": _integer(passports.get("total")),
        "passport_countries": _sorted_strings(passports.get("countries")),
        "passport_jurisdictions": _sorted_strings(passports.get("jurisdictions")),
        "passport_data_residency": _sorted_strings(passports.get("data_residency")),
        "runtime_profiles": _names(
            security.get("runtime_execution_profiles")
            or bytecode.get("runtime_execution_profiles")
        ),
        "sandboxed_adapters": _names(
            security.get("sandboxed_provider_adapters")
            or bytecode.get("sandboxed_provider_adapters")
        ),
        "provider_boundary": boundary,
        "evidence_digests": digest_values,
        "prompt_content_present": prompt_content_present,
        "prompt_text": prompt,
    }


def inventory_sessions(root: Path, prompt_allowlist=None):
    """Return a deterministic inventory of request directories under *root*."""
    prompt_allowlist = frozenset(prompt_allowlist or ())
    root = Path(root)
    if _is_link_or_reparse(root):
        raise ValueError(f"input root must not be a link or reparse point: {root}")
    root = root.resolve(strict=True)
    sessions = [
        _normalize_session(
            root, safe_path, publish_prompt=safe_path.name in prompt_allowlist
        )
        for path in sorted(root.iterdir(), key=lambda item: item.name)
        if (safe_path := _contained_resolved_path(root, path)) is not None
        and safe_path.is_dir()
    ]
    complete = sum(1 for session in sessions if session["complete"])
    inspected = sum(
        session["prompt_content_present"] is not None for session in sessions
    )
    with_content = sum(
        session["prompt_content_present"] is True for session in sessions
    )
    return {
        "total_sessions": len(sessions),
        "complete_sessions": complete,
        "incomplete_sessions": len(sessions) - complete,
        "traces_inspected_for_prompt_content": inspected,
        "traces_with_prompt_content": with_content,
        "sessions": sessions,
    }


def _csv_value(value):
    if isinstance(value, (dict, list)):
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    if value is None:
        return ""
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, str) and value.lstrip().startswith(("=", "+", "-", "@")):
        return "'" + value
    return value


def validate_output_paths(input_root, summary, sessions, events, extra_inputs=()):
    root = Path(input_root).resolve(strict=True)
    outputs = [
        Path(path).resolve(strict=False) for path in (summary, sessions, events)
    ]
    if len(set(outputs)) != len(outputs):
        raise ValueError("output paths must be distinct")
    if any(path == root or path.is_relative_to(root) for path in outputs):
        raise ValueError("output paths must be outside the input root")
    inputs = {Path(path).resolve(strict=True) for path in extra_inputs}
    if any(path in inputs for path in outputs):
        raise ValueError("output paths must not overwrite explicit inputs")


def _write_sessions(path, sessions):
    fields = [
        "request_id", "complete", "artifact_count", "execution_status", "policy_passed",
        "review_required", "policy_violation_count", "policy_violation_rules",
        "policy_violation_reasons", "security_checks", "ledger_events_total",
        "passport_total", "passport_countries", "passport_jurisdictions",
        "passport_data_residency", "runtime_profiles", "sandboxed_adapters",
        "provider_boundary", "evidence_digests", "prompt_content_present",
        "prompt_text", "artifacts",
        "json_errors",
    ]
    def write(temporary):
        with temporary.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields)
            writer.writeheader()
            for session in sessions:
                writer.writerow(
                    {field: _csv_value(session.get(field)) for field in fields}
                )
    atomic_publish(path, write)


def _write_events(path, sessions):
    def write(temporary):
        with temporary.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.writer(handle)
            writer.writerow(("request_id", "event_kind", "count"))
            for session in sessions:
                for kind, count in session["ledger_event_kinds"].items():
                    writer.writerow(
                        (_csv_value(session["request_id"]), _csv_value(kind), count)
                    )
    atomic_publish(path, write)


def load_prompt_allowlist(path):
    if path is None:
        return set()
    value = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise ValueError("prompt allowlist must be a JSON array of request_id strings")
    return set(value)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--sessions", required=True, type=Path)
    parser.add_argument("--events", required=True, type=Path)
    parser.add_argument("--prompt-allowlist", type=Path)
    args = parser.parse_args()

    extra_inputs = [args.prompt_allowlist] if args.prompt_allowlist else []
    validate_output_paths(
        args.input, args.summary, args.sessions, args.events, extra_inputs
    )
    inventory = inventory_sessions(
        args.input, prompt_allowlist=load_prompt_allowlist(args.prompt_allowlist)
    )
    atomic_write_text(
        args.summary,
        json.dumps(inventory, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
    )
    _write_sessions(args.sessions, inventory["sessions"])
    _write_events(args.events, inventory["sessions"])


if __name__ == "__main__":
    main()
