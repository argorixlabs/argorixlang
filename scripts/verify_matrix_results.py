from __future__ import annotations

import csv
import json
import sys

from controlled_matrix_common import COUNTRIES, RESULTS


def fail(message: str) -> None:
    raise SystemExit(f"controlled matrix verification failed: {message}")


def main() -> None:
    required = [
        "controlled_matrix.csv",
        "controlled_matrix.json",
        "controlled_matrix.md",
        "policy_lattice_summary.csv",
        "passport_country_coverage.csv",
        "tamper_results.csv",
        "provider_boundary_results.csv",
    ]
    for name in required:
        if not (RESULTS / name).is_file():
            fail(f"missing output {name}")

    with (RESULTS / "controlled_matrix.csv").open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    by_case = {row["Case"]: row for row in rows}
    if len(rows) < 55:
        fail(f"expected at least 55 rows, got {len(rows)}")
    countries = {code for code, _, _ in COUNTRIES}
    valid = [row for row in rows if row["Case"].startswith("passport_valid_")]
    if len(valid) != len(countries):
        fail(f"expected {len(countries)} valid passport rows, got {len(valid)}")
    if {row["Country"] for row in valid} != countries:
        fail("valid passport country coverage mismatch")
    if any(row["Observed"] != "PASS" for row in valid):
        fail("not every valid passport row passed")

    expectations = {
        "passport_unknown_XX": "REVIEW",
        "passport_unknown_ZZ": "REVIEW",
        "passport_malformed_CHILE": "ERROR",
        "passport_malformed_123": "ERROR",
        "passport_missing_residency_CL": "REVIEW",
        "external_provider_without_adapter": "DENY",
        "network_denied_profile": "DENY",
        "plaintext_secret_attempt": "DENY",
        "key_material_attempt": "DENY",
        "unknown_policy_rule": "UNKNOWN_RULE",
        "malformed_policy_object": "ERROR",
    }
    for case, expected in expectations.items():
        if by_case.get(case, {}).get("Observed") != expected:
            fail(f"{case} observed {by_case.get(case, {}).get('Observed')} not {expected}")
    denied = [row for row in rows if row["Observed"] == "DENY"]
    if any(row["Side Effect"] != "no" for row in denied):
        fail("denied case produced side effect")
    tamper = [row for row in rows if row["Case"].startswith("tampered_") or row["Case"] == "source_mismatch"]
    if len(tamper) != 5 or any(row["Observed"] != "VERIFIER_FAIL" for row in tamper):
        fail("tamper cases did not all fail verification")

    summary = json.loads((RESULTS / "controlled_matrix.json").read_text(encoding="utf-8"))["summary"]
    for key in ("false_allow_count", "false_deny_count", "false_review_count", "external_provider_false_allow_count"):
        if summary.get(key) != 0:
            fail(f"{key} is nonzero: {summary.get(key)}")
    if summary.get("countries_tested") < 30:
        fail("less than 30 countries tested")
    if summary.get("tamper_detection_rate") != 1.0:
        fail("tamper detection rate is not 1.0")
    if summary.get("source_mismatch_detection_rate") != 1.0:
        fail("source mismatch detection rate is not 1.0")
    print(f"Controlled matrix verified: {len(rows)} rows, {summary['countries_tested']} countries")


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(1)
