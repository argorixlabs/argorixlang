#!/usr/bin/env python3
"""Reproducibly inventory Argorix chatbot runtime artifacts."""

import argparse
import csv
import json
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


def _mapping(value):
    return value if isinstance(value, dict) else {}


def _list(value):
    return value if isinstance(value, list) else []


def _sorted_strings(value):
    return sorted(str(item) for item in _list(value))


def _names(value):
    value = _mapping(value)
    if isinstance(value.get("names"), list):
        return _sorted_strings(value["names"])
    return sorted(
        str(item["name"])
        for item in _list(value)
        if isinstance(item, dict) and "name" in item
    )


def _load_json(path, errors):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        errors[path.name] = f"{type(exc).__name__}: {exc}"
        return {}


def _normalize_session(directory):
    artifacts = {}
    for name in ARTIFACT_NAMES:
        path = directory / name
        present = path.is_file()
        artifacts[name] = {
            "present": present,
            "size_bytes": path.stat().st_size if present else 0,
        }

    errors = {}
    documents = {
        name: _load_json(directory / name, errors)
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
    boundary = _mapping(security.get("provider_boundary"))
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
    prompt = injected.get("content")
    if not isinstance(prompt, str):
        prompt = None

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
            str(item["rule"]) for item in violations if item.get("rule") is not None
        ),
        "policy_violation_reasons": sorted(
            {str(item["reason"]) for item in violations if item.get("reason") is not None}
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
    sessions = [
        _normalize_session(path)
        for path in sorted(root.iterdir(), key=lambda item: item.name)
        if path.is_dir()
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
    return value


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
