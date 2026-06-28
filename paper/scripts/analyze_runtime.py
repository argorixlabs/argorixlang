#!/usr/bin/env python3
"""Reproducibly inventory Argorix chatbot runtime artifacts."""

import argparse
import csv
import json
import os
import re
from pathlib import Path


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
    for key, secret in os.environ.items():
        if (
            secret
            and len(secret) >= 4
            and any(marker in key.upper() for marker in ("KEY", "TOKEN", "SECRET", "PASSWORD"))
        ):
            text = text.replace(secret, "[REDACTED]")
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
        "external_contracts_total": (
            boundary.get("external_contracts_total")
            if isinstance(boundary.get("external_contracts_total"), int)
            else None
        ),
        "external_execution_blocked": (
            boundary.get("external_execution_blocked")
            if isinstance(boundary.get("external_execution_blocked"), bool)
            else None
        ),
        "blocked_attempts": (
            boundary.get("blocked_attempts")
            if isinstance(boundary.get("blocked_attempts"), int)
            else None
        ),
    }


def _normalize_session(root, directory):
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
    trace = _mapping(documents.get("session.trace.json"))
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
    event_kinds = {key: event_kinds[key] for key in sorted(event_kinds)}
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
    prompt = _sanitize_text(injected.get("content"), 2000)

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
        "ledger_events_total": ledger.get(
            "events_total", sum(v for v in event_kinds.values() if isinstance(v, int))
        ),
        "ledger_event_kinds": event_kinds,
        "passport_total": passports.get("total"),
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
        "prompt_text": prompt,
    }


def inventory_sessions(root: Path):
    """Return a deterministic inventory of request directories under *root*."""
    root = Path(root)
    if _is_link_or_reparse(root):
        raise ValueError(f"input root must not be a link or reparse point: {root}")
    root = root.resolve(strict=True)
    sessions = [
        _normalize_session(root, safe_path)
        for path in sorted(root.iterdir(), key=lambda item: item.name)
        if (safe_path := _contained_resolved_path(root, path)) is not None
        and safe_path.is_dir()
    ]
    complete = sum(1 for session in sessions if session["complete"])
    return {
        "total_sessions": len(sessions),
        "complete_sessions": complete,
        "incomplete_sessions": len(sessions) - complete,
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


def validate_output_paths(input_root, summary, sessions, events):
    root = Path(input_root).resolve(strict=True)
    outputs = [
        Path(path).resolve(strict=False) for path in (summary, sessions, events)
    ]
    if len(set(outputs)) != len(outputs):
        raise ValueError("output paths must be distinct")
    if any(path == root or path.is_relative_to(root) for path in outputs):
        raise ValueError("output paths must be outside the input root")


def _write_sessions(path, sessions):
    fields = [
        "request_id", "complete", "artifact_count", "execution_status", "policy_passed",
        "review_required", "policy_violation_count", "policy_violation_rules",
        "policy_violation_reasons", "security_checks", "ledger_events_total",
        "passport_total", "passport_countries", "passport_jurisdictions",
        "passport_data_residency", "runtime_profiles", "sandboxed_adapters",
        "provider_boundary", "evidence_digests", "prompt_text", "artifacts",
        "json_errors",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for session in sessions:
            writer.writerow({field: _csv_value(session.get(field)) for field in fields})


def _write_events(path, sessions):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(("request_id", "event_kind", "count"))
        for session in sessions:
            for kind, count in session["ledger_event_kinds"].items():
                writer.writerow((session["request_id"], kind, count))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--sessions", required=True, type=Path)
    parser.add_argument("--events", required=True, type=Path)
    args = parser.parse_args()

    validate_output_paths(args.input, args.summary, args.sessions, args.events)
    inventory = inventory_sessions(args.input)
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    args.summary.write_text(
        json.dumps(inventory, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    _write_sessions(args.sessions, inventory["sessions"])
    _write_events(args.events, inventory["sessions"])


if __name__ == "__main__":
    main()
