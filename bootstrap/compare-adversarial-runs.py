"""Compare declared deterministic, observable outcomes of offline campaign rows.

This deliberately does not compare time, nonce, temporary paths or raw JSON
bytes. E5 (live model) must not be presented as deterministic here.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


FAMILIES = {"E0", "E1", "E2", "E3", "E4", "E6"}
OBSERVED_FIELDS = (
    "outcome",
    "phase_reached",
    "decision_phase",
    "dispatch_outcome",
    "sink_hits",
    "filesystem_hits",
    "secret_hits",
    "unknown_rule_findings",
    "policy_approved",
    "artifact_count",
)


def load(path: Path) -> dict[tuple[str, int], dict]:
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]
    selected = [row for row in rows if row["family"] in FAMILIES]
    if len(selected) != 151:
        raise ValueError(f"{path}: expected 151 offline rows, got {len(selected)}")
    projected = {}
    for row in selected:
        key = row["case_id"], row["repetition"]
        if key in projected:
            raise ValueError(f"{path}: duplicate {key}")
        observed = row.get("observed") or {}
        projected[key] = {
            "family": row["family"],
            "observed": {name: observed.get(name) for name in OBSERVED_FIELDS if name in observed},
            "diagnostic_classes": row.get("diagnostic_classes") or [],
        }
    return projected


def digest(rows: dict[tuple[str, int], dict]) -> str:
    ordered = [{"case_id": key[0], "repetition": key[1], **rows[key]} for key in sorted(rows)]
    data = json.dumps(ordered, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(data).hexdigest().upper()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    parser.add_argument("--historical", type=Path)
    args = parser.parse_args()
    first, second = load(args.first), load(args.second)
    if first != second:
        mismatch = next(key for key in sorted(set(first) | set(second)) if first.get(key) != second.get(key))
        raise ValueError(f"two offline reruns differ at {mismatch}: {first.get(mismatch)} != {second.get(mismatch)}")
    report = {"rows": len(first), "offline_rerun_equal": True, "normalized_sha256": digest(first)}
    if args.historical:
        historical = load(args.historical)
        report["historical_equal"] = historical == first
        report["historical_normalized_sha256"] = digest(historical)
        if historical != first:
            report["historical_first_mismatch"] = str(next(
                key for key in sorted(set(first) | set(historical)) if first.get(key) != historical.get(key)
            ))
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        parser = argparse.ArgumentParser()
        parser.exit(1, f"ESP-002 comparison failed: {error}\n")
