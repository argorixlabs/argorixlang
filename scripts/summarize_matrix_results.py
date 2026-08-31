from __future__ import annotations

import csv
import json
from collections import Counter

from controlled_matrix_common import COUNTRIES, RESULTS, CSV_COLUMNS, write_csv, write_json


def rows() -> list[dict]:
    with (RESULTS / "controlled_matrix.csv").open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def rate(numerator: int, denominator: int) -> float:
    return 0.0 if denominator == 0 else round(numerator / denominator, 6)


def main() -> None:
    data = rows()
    observed = Counter(row["Observed"] for row in data)
    outcome = Counter(row["Outcome"] for row in data)
    valid_passports = [row for row in data if row["Case"].startswith("passport_valid_")]
    tamper = [row for row in data if row["Outcome"] in {"correct_tamper_detection", "correct_source_detection"}]
    source_tamper = [row for row in data if row["Case"] == "source_mismatch"]
    evidence_pass = sum(row["Evidence"] == "bundle_verified" for row in data)
    evidence_fail = len(data) - evidence_pass
    provider_cases = [row for row in data if row["Case"] in {
        "valid_simulated_provider", "external_provider_without_adapter",
        "network_denied_profile", "plaintext_secret_attempt", "key_material_attempt",
    }]

    summary = {
        "countries_tested": len({row["Country"] for row in valid_passports}),
        "valid_passport_cases": len(valid_passports),
        "pass_outcomes": observed["PASS"],
        "deny_outcomes": observed["DENY"],
        "review_outcomes": observed["REVIEW"],
        "unknown_rule_outcomes": observed["UNKNOWN_RULE"],
        "error_outcomes": observed["ERROR"],
        "verifier_fail_outcomes": observed["VERIFIER_FAIL"],
        "false_allow_count": outcome["false_allow"],
        "false_deny_count": outcome["false_deny"],
        "false_review_count": outcome["false_review"],
        "tamper_detection_rate": rate(len([row for row in tamper if row["Case"] != "source_mismatch"]), 4),
        "source_mismatch_detection_rate": rate(len([row for row in source_tamper if row["Observed"] == "VERIFIER_FAIL"]), len(source_tamper)),
        "external_provider_false_allow_count": len([row for row in provider_cases if row["Case"] == "external_provider_without_adapter" and row["Observed"] == "PASS"]),
        "evidence_verification_pass_count": evidence_pass,
        "evidence_verification_fail_count": evidence_fail,
        "total_rows": len(data),
    }
    payload = json.loads((RESULTS / "controlled_matrix.json").read_text(encoding="utf-8"))
    payload["summary"] = summary
    write_json(RESULTS / "controlled_matrix.json", payload)

    coverage_rows = []
    for code, name, region in COUNTRIES:
        case = f"passport_valid_{code}_residency_{code}"
        match = next(row for row in data if row["Case"] == case)
        coverage_rows.append({
            "country_code": code,
            "country_name": name,
            "region_group": region,
            "valid_same_country_case": str(match["Observed"] == "PASS").lower(),
            "observed": match["Observed"],
            "source": match["Source"],
        })
    write_csv(RESULTS / "passport_country_coverage.csv", coverage_rows, [
        "country_code", "country_name", "region_group", "valid_same_country_case", "observed", "source",
    ])

    lattice_rows = [
        {"outcome": key, "count": observed[key]}
        for key in ("PASS", "DENY", "REVIEW", "UNKNOWN_RULE", "ERROR", "VERIFIER_FAIL")
    ]
    write_csv(RESULTS / "policy_lattice_summary.csv", lattice_rows, ["outcome", "count"])

    tamper_rows = [
        row for row in data if row["Case"].startswith("tampered_") or row["Case"] == "source_mismatch"
    ]
    write_csv(RESULTS / "tamper_results.csv", tamper_rows, CSV_COLUMNS)
    write_csv(RESULTS / "provider_boundary_results.csv", provider_cases, CSV_COLUMNS)
    print(f"Summarized {len(data)} rows: {summary}")


if __name__ == "__main__":
    main()
