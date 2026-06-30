from __future__ import annotations

import time

from controlled_matrix_common import COUNTRIES, EXPERIMENT, write_json


def fixture(case: str, group: str, **fields) -> dict:
    return {
        "case": case,
        "group": group,
        "country": fields.get("country"),
        "residency": fields.get("residency"),
        "data_class": fields.get("data_class", "internal"),
        "provider": fields.get("provider", "simulated"),
        "profile": fields.get("profile", "default"),
        "policy_rule": fields.get("policy_rule"),
        "tamper_target": fields.get("tamper_target"),
        "expected": fields.get("expected"),
        "expected_policy": fields.get("expected_policy"),
        "note": fields.get("note", ""),
    }


def fixtures() -> list[dict]:
    cases: list[dict] = []
    for code, name, region in COUNTRIES:
        cases.append(
            fixture(
                f"passport_valid_{code}_residency_{code}",
                "passports",
                country=code,
                residency=code,
                data_class="internal",
                expected="PASS",
                expected_policy=(
                    "PASS: passport_country_code_valid + "
                    "passport_residency_declared + same_country_residency"
                ),
                note=f"Valid same-country passport fixture for {name} ({region}).",
            )
        )

    cases.extend(
        [
            fixture("passport_CL_residency_US_sensitive", "passports", country="CL", residency="US", data_class="sensitive", expected="REVIEW", expected_policy="REVIEW: cross_border_sensitive_data"),
            fixture("passport_CL_residency_EU_sensitive", "passports", country="CL", residency="EU", data_class="sensitive", expected="REVIEW", expected_policy="REVIEW: cross_border_sensitive_data", note="EU is a regional residency zone, not a country."),
            fixture("passport_CL_residency_BR_public", "passports", country="CL", residency="BR", data_class="public", expected="PASS", expected_policy="PASS: allowed_foreign_residency"),
            fixture("passport_CL_residency_CN_confidential", "passports", country="CL", residency="CN", data_class="confidential", profile="cl_block_cn", expected="DENY", expected_policy="DENY: disallowed_residency"),
            fixture("passport_US_residency_EU_regulated", "passports", country="US", residency="EU", data_class="regulated", expected="REVIEW", expected_policy="REVIEW: cross_border_sensitive_data"),
            fixture("passport_EU_member_ES_residency_DE", "passports", country="ES", residency="DE", data_class="internal", profile="eu_internal_allowed", expected="PASS", expected_policy="PASS: allowed_eu_internal_residency"),
            fixture("passport_unknown_XX", "passports", country="XX", residency="XX", data_class="internal", expected="REVIEW", expected_policy="REVIEW: unknown_jurisdiction"),
            fixture("passport_unknown_ZZ", "passports", country="ZZ", residency="ZZ", data_class="internal", expected="REVIEW", expected_policy="REVIEW: unknown_jurisdiction"),
            fixture("passport_malformed_CHILE", "passports", country="CHILE", residency="CL", data_class="internal", expected="ERROR", expected_policy="ERROR: malformed_country_code"),
            fixture("passport_malformed_EUROPE", "passports", country="EUROPE", residency="EU", data_class="internal", expected="ERROR", expected_policy="ERROR: malformed_country_code"),
            fixture("passport_malformed_123", "passports", country="123", residency="CL", data_class="internal", expected="ERROR", expected_policy="ERROR: malformed_country_code"),
            fixture("passport_malformed_empty", "passports", country="", residency="CL", data_class="internal", expected="ERROR", expected_policy="ERROR: malformed_country_code"),
            fixture("passport_null_country", "passports", country=None, residency="CL", data_class="internal", expected="ERROR", expected_policy="ERROR: malformed_country_code"),
            fixture("passport_missing_residency_CL", "passports", country="CL", residency=None, data_class="internal", expected="REVIEW", expected_policy="REVIEW: passport_residency_missing"),
        ]
    )

    cases.extend(
        [
            fixture("valid_simulated_provider", "providers", country="CL", residency="CL", provider="simulated", expected="PASS", expected_policy="PASS: provider_executable_allowlist"),
            fixture("external_provider_without_adapter", "providers", country="CL", residency="CL", provider="external", expected="DENY", expected_policy="DENY: external_provider_without_adapter"),
            fixture("network_denied_profile", "providers", country="CL", residency="CL", provider="network", expected="DENY", expected_policy="DENY: runtime_network_denied"),
            fixture("plaintext_secret_attempt", "providers", country="CL", residency="CL", provider="secret", expected="DENY", expected_policy="DENY: secrets_denied"),
            fixture("key_material_attempt", "providers", country="CL", residency="CL", provider="key_material", expected="DENY", expected_policy="DENY: key_material_denied"),
        ]
    )

    cases.extend(
        [
            fixture("tampered_trace", "tamper", country="CL", residency="CL", expected="VERIFIER_FAIL", expected_policy="n/a", tamper_target="trace"),
            fixture("tampered_report", "tamper", country="CL", residency="CL", expected="VERIFIER_FAIL", expected_policy="n/a", tamper_target="report"),
            fixture("tampered_bytecode", "tamper", country="CL", residency="CL", expected="VERIFIER_FAIL", expected_policy="n/a", tamper_target="bytecode"),
            fixture("tampered_ledger_events", "tamper", country="CL", residency="CL", expected="VERIFIER_FAIL", expected_policy="n/a", tamper_target="ledger"),
            fixture("source_mismatch", "tamper", country="CL", residency="CL", expected="VERIFIER_FAIL", expected_policy="n/a", tamper_target="source"),
        ]
    )

    cases.extend(
        [
            fixture("unknown_policy_rule", "policies", country="CL", residency="CL", expected="UNKNOWN_RULE", expected_policy="UNKNOWN_RULE: unsupported_policy_identifier", policy_rule="unsupported_policy_identifier"),
            fixture("malformed_policy_object", "policies", country="CL", residency="CL", expected="ERROR", expected_policy="ERROR: malformed_policy_object", policy_rule="malformed_policy_object"),
        ]
    )
    return cases


def main() -> None:
    generated = fixtures()
    expected_paths = {
        EXPERIMENT / case["group"] / f"{case['case']}.json"
        for case in generated
    }
    for category in ("passports", "providers", "tamper", "policies"):
        directory = EXPERIMENT / category
        directory.mkdir(parents=True, exist_ok=True)
        for old in directory.glob("*.json"):
            if old in expected_paths:
                continue
            for attempt in range(10):
                try:
                    old.unlink()
                    break
                except PermissionError:
                    if attempt == 9:
                        raise
                    time.sleep(0.1)

    for case in generated:
        write_json(EXPERIMENT / case["group"] / f"{case['case']}.json", case)
    print(f"Generated {len(generated)} controlled-matrix fixtures under {EXPERIMENT}")


if __name__ == "__main__":
    main()
