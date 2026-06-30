import csv
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results"
COUNTRIES = {
    "CL", "AR", "BR", "UY", "PE", "CO", "MX", "US", "CA", "ES", "FR",
    "DE", "IT", "NL", "SE", "NO", "GB", "PT", "IE", "JP", "KR", "SG",
    "IN", "CN", "AU", "NZ", "ZA", "NG", "KE", "AE", "IL",
}


def run_pipeline():
    for script in (
        "generate_passport_matrix.py",
        "run_controlled_matrix.py",
        "summarize_matrix_results.py",
        "verify_matrix_results.py",
    ):
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / script)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        assert completed.returncode == 0, completed.stdout + completed.stderr


def read_rows():
    with (RESULTS / "controlled_matrix.csv").open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def test_controlled_matrix_acceptance_contract():
    run_pipeline()
    rows = read_rows()
    assert len(rows) >= 55
    assert {row["Country"] for row in rows if row["Case"].startswith("passport_valid_")} == COUNTRIES

    valid = [row for row in rows if row["Case"].startswith("passport_valid_")]
    assert len(valid) == 31
    assert all(row["Expected"] == "PASS" and row["Observed"] == "PASS" for row in valid)
    assert all(row["Source"] == "source_digest_match" for row in valid)

    by_case = {row["Case"]: row for row in rows}
    assert by_case["passport_unknown_XX"]["Observed"] == "REVIEW"
    assert by_case["passport_unknown_ZZ"]["Policy"].startswith("REVIEW: unknown_jurisdiction")
    assert by_case["passport_malformed_CHILE"]["Observed"] == "ERROR"
    assert by_case["passport_null_country"]["Policy"].startswith("ERROR: malformed_country_code")
    assert by_case["passport_missing_residency_CL"]["Observed"] == "REVIEW"
    assert by_case["external_provider_without_adapter"]["Observed"] == "DENY"
    assert by_case["external_provider_without_adapter"]["Provider"] == "external_blocked_no_adapter"
    assert by_case["passport_CL_residency_CN_confidential"]["Observed"] == "DENY"
    assert by_case["passport_EU_member_ES_residency_DE"]["Observed"] == "PASS"

    denied = [row for row in rows if row["Observed"] == "DENY"]
    assert denied
    assert all(row["Side Effect"] == "no" for row in denied)

    tamper = [row for row in rows if row["Case"].startswith(("tampered_", "source_mismatch"))]
    assert len(tamper) == 5
    assert all(row["Observed"] == "VERIFIER_FAIL" for row in tamper)
    assert {row["Outcome"] for row in tamper} == {"correct_tamper_detection", "correct_source_detection"}

    unknown = by_case["unknown_policy_rule"]
    assert unknown["Observed"] == "UNKNOWN_RULE"
    assert unknown["Policy"] == "UNKNOWN_RULE: unsupported_policy_identifier"

    malformed_policy = by_case["malformed_policy_object"]
    assert malformed_policy["Observed"] == "ERROR"
    assert malformed_policy["Policy"] == "ERROR: malformed_policy_object"


def test_matrix_summary_and_coverage_outputs_are_consistent():
    run_pipeline()
    summary = json.loads((RESULTS / "controlled_matrix.json").read_text(encoding="utf-8"))["summary"]
    assert summary["countries_tested"] == 31
    assert summary["valid_passport_cases"] == 31
    assert summary["false_allow_count"] == 0
    assert summary["false_deny_count"] == 0
    assert summary["false_review_count"] == 0
    assert summary["external_provider_false_allow_count"] == 0
    assert summary["tamper_detection_rate"] == 1.0
    assert summary["source_mismatch_detection_rate"] == 1.0

    coverage = list(csv.DictReader((RESULTS / "passport_country_coverage.csv").open(encoding="utf-8")))
    assert len(coverage) == 31
    assert {row["country_code"] for row in coverage} == COUNTRIES
    assert all(row["valid_same_country_case"] == "true" for row in coverage)

    for required in (
        "controlled_matrix.md",
        "policy_lattice_summary.csv",
        "tamper_results.csv",
        "provider_boundary_results.csv",
    ):
        assert (RESULTS / required).is_file()
