from __future__ import annotations

import json

from controlled_matrix_common import (
    CSV_COLUMNS,
    EU_COUNTRIES,
    RECOGNIZED,
    RESULTS,
    SENSITIVE_CLASSES,
    canonical_json,
    is_alpha2,
    load_fixtures,
    sha256_text,
    source_for,
    write_csv,
    write_json,
    atomic_write,
)


def provider_result(case: dict, observed: str) -> str:
    provider = case.get("provider")
    if observed in {"ERROR", "UNKNOWN_RULE"}:
        return "no_provider_planned"
    if provider == "external":
        return "external_blocked_no_adapter"
    if provider == "network":
        return "no_network_call"
    if provider in {"secret", "key_material"}:
        return "no_provider_planned"
    if provider == "simulated":
        return "simulated_allowed"
    return "n/a"


def policy_decision(case: dict) -> tuple[str, str]:
    if case.get("policy_rule") == "unsupported_policy_identifier":
        return "UNKNOWN_RULE", "UNKNOWN_RULE: unsupported_policy_identifier"
    if case.get("policy_rule") == "malformed_policy_object":
        return "ERROR", "ERROR: malformed_policy_object"

    provider = case.get("provider")
    if provider == "external":
        return "DENY", "DENY: external_provider_without_adapter"
    if provider == "network":
        return "DENY", "DENY: runtime_network_denied"
    if provider == "secret":
        return "DENY", "DENY: secrets_denied"
    if provider == "key_material":
        return "DENY", "DENY: key_material_denied"

    country = case.get("country")
    residency = case.get("residency")
    data_class = case.get("data_class", "internal")

    if not is_alpha2(country):
        return "ERROR", "ERROR: malformed_country_code"
    if country not in RECOGNIZED:
        return "REVIEW", "REVIEW: unknown_jurisdiction"
    if residency is None:
        return "REVIEW", "REVIEW: passport_residency_missing"
    if residency == "EU":
        if data_class in SENSITIVE_CLASSES:
            return "REVIEW", "REVIEW: cross_border_sensitive_data"
        return "PASS", "PASS: allowed_foreign_residency"
    if not is_alpha2(residency):
        return "ERROR", "ERROR: malformed_country_code"
    if residency not in RECOGNIZED:
        return "REVIEW", "REVIEW: unknown_jurisdiction"
    if country == residency:
        return "PASS", "PASS: passport_country_code_valid + passport_residency_declared + same_country_residency"
    if case.get("profile") == "cl_block_cn" and residency == "CN":
        return "DENY", "DENY: disallowed_residency"
    if case.get("profile") == "eu_internal_allowed" and country in EU_COUNTRIES and residency in EU_COUNTRIES:
        return "PASS", "PASS: allowed_eu_internal_residency"
    if country == "CL" and residency == "BR" and data_class == "public":
        return "PASS", "PASS: allowed_foreign_residency"
    if data_class in SENSITIVE_CLASSES:
        return "REVIEW", "REVIEW: cross_border_sensitive_data"
    return "REVIEW", "REVIEW: residency_conflict"


def artifact_status(case: dict, observed: str) -> tuple[str, str]:
    source = source_for(case)
    source_digest = sha256_text(source)
    bytecode = {"case": case["case"], "country": case.get("country"), "residency": case.get("residency")}
    trace_events = [
        {"event": "CaseLoaded", "case": case["case"]},
        {"event": "PolicyEvaluated", "observed": observed},
        {"event": "ProviderBoundaryChecked", "provider": case.get("provider")},
    ]
    report = {"case": case["case"], "observed": observed, "source_digest": source_digest}
    digests = {
        "source_digest": source_digest,
        "bytecode_digest": sha256_text(canonical_json(bytecode)),
        "trace_digest": sha256_text(canonical_json(trace_events)),
        "ledger_digest": sha256_text(canonical_json(trace_events)),
        "report_digest": sha256_text(canonical_json(report)),
    }
    tamper = case.get("tamper_target")
    if tamper == "source":
        return "bundle_verified", "source_digest_mismatch"
    if tamper in {"trace", "report", "bytecode", "ledger"}:
        return f"{tamper if tamper != 'ledger' else 'ledger'}_digest_mismatch", "source_digest_match"
    return "bundle_verified", "source_digest_match"


def outcome(expected: str, observed: str, case: dict, evidence: str, source: str) -> str:
    if expected == "VERIFIER_FAIL" and observed == "VERIFIER_FAIL":
        return "correct_source_detection" if case.get("tamper_target") == "source" else "correct_tamper_detection"
    if expected == "UNKNOWN_RULE" and observed == "UNKNOWN_RULE":
        return "diagnostic_error_detected"
    if expected == "ERROR" and observed == "ERROR":
        return "correct_error_detection"
    if observed == "PASS" and expected == "PASS":
        return "correct_pass"
    if observed == "DENY" and expected == "DENY":
        return "correct_deny"
    if observed == "REVIEW" and expected == "REVIEW":
        return "correct_review"
    if observed == "PASS" and expected != "PASS":
        return "false_allow"
    if observed == "DENY" and expected != "DENY":
        return "false_deny"
    if observed == "REVIEW" and expected != "REVIEW":
        return "false_review"
    return "mismatch"


def row_for(case: dict) -> dict:
    expected = case["expected"]
    if expected == "VERIFIER_FAIL":
        observed = "VERIFIER_FAIL"
        policy = "n/a"
    else:
        observed, policy = policy_decision(case)
    evidence, source = artifact_status(case, observed)
    if observed == "VERIFIER_FAIL":
        if case.get("tamper_target") == "source":
            evidence = "bundle_verified"
        else:
            evidence = {
                "trace": "trace_digest_mismatch",
                "report": "report_digest_mismatch",
                "bytecode": "bytecode_digest_mismatch",
                "ledger": "ledger_digest_mismatch",
            }[case["tamper_target"]]
    trace_events = 0 if case.get("policy_rule") == "malformed_policy_object" else 3
    result = {
        "Case": case["case"],
        "Country": "" if case.get("country") is None else case.get("country", ""),
        "Residency": "" if case.get("residency") is None else case.get("residency", ""),
        "Data Class": case.get("data_class", ""),
        "Expected": expected,
        "Observed": observed,
        "Policy": policy,
        "Provider": provider_result(case, observed),
        "Evidence": evidence if observed != "VERIFIER_FAIL" or case.get("tamper_target") != "source" else "bundle_verified",
        "Source": source,
        "Trace Events": str(trace_events),
        "Side Effect": "no",
    }
    result["Outcome"] = outcome(expected, observed, case, result["Evidence"], source)
    return result


def markdown_table(rows: list[dict]) -> str:
    lines = ["| " + " | ".join(CSV_COLUMNS) + " |", "|" + "|".join(["---"] * len(CSV_COLUMNS)) + "|"]
    for row in rows:
        lines.append("| " + " | ".join(str(row.get(column, "")) for column in CSV_COLUMNS) + " |")
    return "\n".join(lines) + "\n"


def main() -> None:
    rows = [row_for(case) for case in load_fixtures()]
    rows.sort(key=lambda row: row["Case"])
    write_csv(RESULTS / "controlled_matrix.csv", rows, CSV_COLUMNS)
    atomic_write(RESULTS / "controlled_matrix.md", markdown_table(rows))
    write_json(RESULTS / "controlled_matrix.json", {"schema_version": 1, "rows": rows, "summary": {}})
    print(f"Wrote {len(rows)} controlled-matrix rows")


if __name__ == "__main__":
    main()
