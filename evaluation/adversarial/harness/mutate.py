"""Post-generation mutations for E3.

Every mutation runs *after* a clean evidence set has been produced and verified
by the real binaries.  Mutations edit bytes on disk; they never edit an
in-memory value that is later serialised by the code under test, and they never
consult expected outcomes.
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any, Callable

MutationResult = dict[str, Any]


class MutationNotApplicable(RuntimeError):
    """Raised when the clean set does not contain what a mutation needs."""


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------


def _load(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _store(path: Path, value: Any) -> None:
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def _truncate(path: Path, keep_fraction: float = 0.6) -> dict[str, Any]:
    data = path.read_bytes()
    keep = max(1, int(len(data) * keep_fraction))
    path.write_bytes(data[:keep])
    return {"original_bytes": len(data), "kept_bytes": keep}


def _first_nested(document: Any, keys: list[str]) -> Any:
    cursor = document
    for key in keys:
        if isinstance(cursor, dict) and key in cursor:
            cursor = cursor[key]
        else:
            raise MutationNotApplicable(f"missing key path {'/'.join(keys)}")
    return cursor


class CleanSet:
    """Paths of one generated evidence set inside a mutation workdir."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.source = root / "case.argx"
        self.bytecode = root / "case.argbc.json"
        self.trace = root / "case.trace.json"
        self.report = root / "case.security.json"
        self.bundle = root / "case.bundle.json"

    def copy_to(self, destination: Path) -> "CleanSet":
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(self.root, destination)
        return CleanSet(destination)


# --------------------------------------------------------------------------
# mutation catalogue
# --------------------------------------------------------------------------


def m_bytecode_semantic_value(clean: CleanSet) -> MutationResult:
    document = _load(clean.bytecode)
    before = document.get("module")
    document["module"] = f"{before}.tampered"
    _store(clean.bytecode, document)
    return {"field": "module", "before": before, "after": document["module"]}


def m_bytecode_field_removed(clean: CleanSet) -> MutationResult:
    document = _load(clean.bytecode)
    if "assertions" in document:
        removed = "assertions"
    elif "capabilities" in document:
        removed = "capabilities"
    else:
        raise MutationNotApplicable("no removable bytecode field")
    document.pop(removed)
    _store(clean.bytecode, document)
    return {"field": removed, "action": "removed"}


def m_bytecode_truncated(clean: CleanSet) -> MutationResult:
    return {"action": "truncate", **_truncate(clean.bytecode)}


def m_trace_event_altered(clean: CleanSet) -> MutationResult:
    document = _load(clean.trace)
    events = document.get("events") or []
    if not events:
        raise MutationNotApplicable("trace has no events")
    before = events[0].get("details")
    events[0]["details"] = "tampered event details"
    _store(clean.trace, document)
    return {"event_index": 0, "before": before, "after": events[0]["details"]}


def m_trace_ledger_link_altered(clean: CleanSet) -> MutationResult:
    document = _load(clean.trace)
    events = document.get("events") or []
    if len(events) < 2:
        raise MutationNotApplicable("trace has fewer than two events")
    removed = events.pop(1)
    _store(clean.trace, document)
    return {"action": "event_removed", "event_index": 1, "event_type": removed.get("event_type")}


def m_trace_truncated(clean: CleanSet) -> MutationResult:
    return {"action": "truncate", **_truncate(clean.trace)}


def m_report_policy_result(clean: CleanSet) -> MutationResult:
    document = _load(clean.report)
    policy = _first_nested(document, ["policy"])
    before = policy.get("passed")
    policy["passed"] = not before
    if "review_required" in policy:
        policy["review_required"] = False
    verdict = document.get("verdict")
    if isinstance(verdict, dict):
        verdict["passed"] = True
        verdict["severity"] = "pass"
        verdict["reasons"] = ["policy passed"]
    _store(clean.report, document)
    return {"field": "policy.passed", "before": before, "after": policy["passed"]}


def m_report_ledger_digest(clean: CleanSet) -> MutationResult:
    document = _load(clean.report)
    ledger = _first_nested(document, ["ledger"])
    before = ledger.get("ledger_digest")
    ledger["ledger_digest"] = "sha256:" + "0" * 64
    _store(clean.report, document)
    return {"field": "ledger.ledger_digest", "before": before, "after": ledger["ledger_digest"]}


def m_report_version(clean: CleanSet) -> MutationResult:
    document = _load(clean.report)
    before = document.get("report_version")
    document["report_version"] = "0.0-tampered"
    _store(clean.report, document)
    return {"field": "report_version", "before": before, "after": document["report_version"]}


def m_bundle_digest(clean: CleanSet) -> MutationResult:
    document = _load(clean.bundle)
    before = document.get("bytecode_digest")
    document["bytecode_digest"] = "sha256:" + "1" * 64
    _store(clean.bundle, document)
    return {"field": "bytecode_digest", "before": before, "after": document["bytecode_digest"]}


def m_bundle_path(clean: CleanSet) -> MutationResult:
    document = _load(clean.bundle)
    artifacts = _first_nested(document, ["artifacts"])
    before = artifacts.get("trace_path")
    artifacts["trace_path"] = "case.renamed-trace.json"
    _store(clean.bundle, document)
    return {"field": "artifacts.trace_path", "before": before, "after": artifacts["trace_path"]}


def m_bundle_version(clean: CleanSet) -> MutationResult:
    document = _load(clean.bundle)
    before = document.get("bundle_version")
    document["bundle_version"] = "9.99"
    _store(clean.bundle, document)
    return {"field": "bundle_version", "before": before, "after": document["bundle_version"]}


def m_bundle_trace_relation(clean: CleanSet) -> MutationResult:
    """Keep the trace path but drop the trace digest it must agree with."""
    document = _load(clean.bundle)
    before = document.get("trace_digest")
    document["trace_digest"] = None
    _store(clean.bundle, document)
    return {"field": "trace_digest", "before": before, "after": None}


def m_missing_bytecode_artifact(clean: CleanSet) -> MutationResult:
    clean.bytecode.unlink()
    return {"action": "deleted", "artifact": clean.bytecode.name}


def m_missing_trace_artifact(clean: CleanSet) -> MutationResult:
    clean.trace.unlink()
    return {"action": "deleted", "artifact": clean.trace.name}


def m_missing_report_artifact(clean: CleanSet) -> MutationResult:
    clean.report.unlink()
    return {"action": "deleted", "artifact": clean.report.name}


def m_invalid_json_bytecode(clean: CleanSet) -> MutationResult:
    clean.bytecode.write_text("{ this is not json", encoding="utf-8")
    return {"action": "invalid_json", "artifact": clean.bytecode.name}


def m_invalid_json_trace(clean: CleanSet) -> MutationResult:
    clean.trace.write_text("[[[", encoding="utf-8")
    return {"action": "invalid_json", "artifact": clean.trace.name}


def m_path_outside_portable_tree(clean: CleanSet) -> MutationResult:
    document = _load(clean.bundle)
    artifacts = _first_nested(document, ["artifacts"])
    before = artifacts.get("bytecode_path")
    artifacts["bytecode_path"] = "../../../etc/argorix-escape.json"
    _store(clean.bundle, document)
    return {"field": "artifacts.bytecode_path", "before": before, "after": artifacts["bytecode_path"]}


def m_source_only(clean: CleanSet) -> MutationResult:
    """Modify only the source file.

    The studied bundle schema records no source digest, so this is expected to
    remain undetected.  It is included precisely to document that boundary.
    """
    if not clean.source.exists():
        raise MutationNotApplicable("clean set has no source file")
    text = clean.source.read_text(encoding="utf-8")
    clean.source.write_text(text + "\n// tampered source comment\n", encoding="utf-8")
    return {"action": "source_appended", "artifact": clean.source.name}


def m_full_unsigned_replacement(clean: CleanSet) -> MutationResult:
    """Replace bundle and every artifact with a coordinated, self-consistent set.

    The replacement set is regenerated from a different program by the caller
    and copied over the clean set, so all digests agree internally.  Without a
    signature or trust anchor this is expected to pass verification, which is
    the authenticity limit being documented.
    """
    replacement = clean.root.parent / "replacement-set"
    if not replacement.is_dir():
        raise MutationNotApplicable("replacement set was not generated")
    for name in (
        "case.argx",
        "case.argbc.json",
        "case.trace.json",
        "case.security.json",
        "case.bundle.json",
    ):
        candidate = replacement / name
        if candidate.exists():
            shutil.copy2(candidate, clean.root / name)
    return {"action": "coordinated_replacement", "source": str(replacement)}


MUTATIONS: dict[str, tuple[str, Callable[[CleanSet], MutationResult]]] = {
    "bytecode_semantic_value": ("bytecode", m_bytecode_semantic_value),
    "bytecode_field_removed": ("bytecode", m_bytecode_field_removed),
    "bytecode_truncated": ("bytecode", m_bytecode_truncated),
    "trace_event_altered": ("trace", m_trace_event_altered),
    "trace_ledger_link_altered": ("trace", m_trace_ledger_link_altered),
    "trace_truncated": ("trace", m_trace_truncated),
    "report_policy_result": ("report", m_report_policy_result),
    "report_ledger_digest": ("report", m_report_ledger_digest),
    "report_version": ("report", m_report_version),
    "bundle_digest": ("bundle", m_bundle_digest),
    "bundle_path": ("bundle", m_bundle_path),
    "bundle_version": ("bundle", m_bundle_version),
    "bundle_trace_relation": ("bundle", m_bundle_trace_relation),
    "missing_bytecode_artifact": ("missing_artifact", m_missing_bytecode_artifact),
    "missing_trace_artifact": ("missing_artifact", m_missing_trace_artifact),
    "missing_report_artifact": ("missing_artifact", m_missing_report_artifact),
    "invalid_json_bytecode": ("invalid_json", m_invalid_json_bytecode),
    "invalid_json_trace": ("invalid_json", m_invalid_json_trace),
    "path_outside_portable_tree": ("portable_tree", m_path_outside_portable_tree),
    "source_only": ("source_binding", m_source_only),
    "full_unsigned_replacement": ("authenticity", m_full_unsigned_replacement),
    "bundle_module_identity": ("bundle", None),  # replaced below
}


def m_bundle_module_identity(clean: CleanSet) -> MutationResult:
    document = _load(clean.bundle)
    before = document.get("module")
    document["module"] = f"{before}.rebranded"
    _store(clean.bundle, document)
    return {"field": "module", "before": before, "after": document["module"]}


MUTATIONS["bundle_module_identity"] = ("bundle", m_bundle_module_identity)


def apply_mutation(name: str, clean: CleanSet) -> MutationResult:
    if name not in MUTATIONS:
        raise KeyError(f"unknown mutation `{name}`")
    mutation_class, function = MUTATIONS[name]
    detail = function(clean)
    return {"mutation": name, "mutation_class": mutation_class, "detail": detail}


def mutation_class(name: str) -> str:
    return MUTATIONS[name][0]


__all__ = ["CleanSet", "MUTATIONS", "MutationNotApplicable", "apply_mutation", "mutation_class"]
