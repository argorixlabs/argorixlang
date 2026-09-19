#!/usr/bin/env python3
"""Validate MAT-003's human-authored normative contract against stage0.

The manifest is the oracle. This runner checks its completeness and then treats
the selected compiler as an implementation under test.
"""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import pathlib
import re
import subprocess
import sys
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "conformance" / "normative" / "manifest.json"
COVERAGE = ROOT / "spec" / "language" / "coverage.json"
SPEC = ROOT / "spec" / "language" / "current-v1.md"


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def rust_block(text: str, marker: str) -> str:
    start = text.find(marker)
    if start < 0:
        raise ValueError(f"missing Rust declaration: {marker}")
    brace = text.find("{", start)
    depth = 0
    for offset in range(brace, len(text)):
        if text[offset] == "{":
            depth += 1
        elif text[offset] == "}":
            depth -= 1
            if depth == 0:
                return text[brace + 1 : offset]
    raise ValueError(f"unterminated Rust declaration: {marker}")


def enum_variants(path: pathlib.Path, enum_name: str) -> list[str]:
    body = rust_block(path.read_text(encoding="utf-8"), f"pub enum {enum_name}")
    variants: list[str] = []
    depth = 0
    for line in body.splitlines():
        stripped = line.strip()
        if depth == 0:
            match = re.match(r"([A-Z][A-Za-z0-9_]*)\s*(?:[({,]|$)", stripped)
            if match:
                variants.append(match.group(1))
        depth += line.count("{") + line.count("(")
        depth -= line.count("}") + line.count(")")
    return variants


def program_fields(path: pathlib.Path) -> list[str]:
    body = rust_block(path.read_text(encoding="utf-8"), "pub struct Program")
    return re.findall(r"^\s*pub\s+([a-z][a-z0-9_]*):", body, flags=re.MULTILINE)


def validate_contract(manifest: dict[str, Any], coverage: dict[str, Any]) -> dict[str, Any]:
    problems: list[str] = []
    clauses = {item["id"]: item for item in coverage["clauses"]}
    expected_clauses = {f"L-{number:02d}" for number in range(1, 9)}
    if set(clauses) != expected_clauses:
        problems.append(f"clause set differs: {sorted(set(clauses) ^ expected_clauses)}")

    actual_fields = program_fields(ROOT / "crates" / "argorix_parser" / "src" / "ast.rs")
    covered_fields = [field for clause in coverage["clauses"] for field in clause["fields"]]
    if len(covered_fields) != len(set(covered_fields)):
        problems.append("a Program field appears in more than one clause")
    if set(actual_fields) != set(covered_fields):
        problems.append(
            f"Program coverage mismatch missing={sorted(set(actual_fields)-set(covered_fields))} "
            f"extra={sorted(set(covered_fields)-set(actual_fields))}"
        )

    closed_actual = {
        "token_kinds": enum_variants(ROOT / "crates" / "argorix_parser" / "src" / "lexer.rs", "TokenKind"),
        "handler_instructions": enum_variants(ROOT / "crates" / "argorix_parser" / "src" / "ast.rs", "HandlerInstruction"),
        "bytecode_instructions": enum_variants(ROOT / "crates" / "argorix_bytecode" / "src" / "bytecode.rs", "Instruction"),
    }
    for name, actual in closed_actual.items():
        declared = coverage["closed_sets"].get(name, [])
        if actual != declared:
            problems.append(f"closed set {name} differs: actual={actual} declared={declared}")

    excluded = coverage.get("excluded_from_v1", [])
    if not excluded or any(not item.get("owner") or not item.get("reason") for item in excluded):
        problems.append("every v1 exclusion must have an owner and reason")

    spec_text = SPEC.read_text(encoding="utf-8")
    for clause_id in expected_clauses:
        if f"## {clause_id} " not in spec_text and f"## {clause_id} —" not in spec_text:
            problems.append(f"specification has no heading for {clause_id}")

    ids: set[str] = set()
    kinds_by_clause = {clause_id: set() for clause_id in expected_clauses}
    for case in manifest["cases"]:
        if case["id"] in ids:
            problems.append(f"duplicate case id {case['id']}")
        ids.add(case["id"])
        if case["clause"] not in expected_clauses:
            problems.append(f"unknown clause in {case['id']}")
        else:
            kinds_by_clause[case["clause"]].add(case["kind"])
        source = ROOT / case["source"]
        if not source.is_file():
            problems.append(f"missing source {case['source']}")
        elif sha256(source) != case["sha256"]:
            problems.append(f"source digest drift {case['id']}")
        if not case.get("oracle_basis"):
            problems.append(f"missing independent oracle basis {case['id']}")
    for clause_id, kinds in kinds_by_clause.items():
        if kinds != {"valid", "invalid"}:
            problems.append(f"{clause_id} requires valid and invalid cases; found {sorted(kinds)}")

    for case in manifest.get("planned_core_cases", []):
        if case.get("status") != "PLANNED":
            problems.append(f"Core case {case.get('id')} must remain PLANNED until MAT-004")
        if not (ROOT / case["source"]).is_file():
            problems.append(f"missing planned Core source {case['source']}")

    return {
        "ok": not problems,
        "problems": problems,
        "program_fields_actual": len(actual_fields),
        "program_fields_covered": len(covered_fields),
        "closed_sets": {name: len(values) for name, values in closed_actual.items()},
        "excluded_from_v1": len(excluded),
    }


def compare(case: dict[str, Any], returncode: int, output: str) -> tuple[bool, list[str]]:
    reasons: list[str] = []
    if returncode != case["expected_exit"]:
        reasons.append(f"exit expected {case['expected_exit']} got {returncode}")
    if case["expected_contains"] not in output:
        reasons.append(f"missing diagnostic marker {case['expected_contains']!r}")
    return not reasons, reasons


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    coverage = json.loads(COVERAGE.read_text(encoding="utf-8"))
    compiler = pathlib.Path(args.compiler).resolve()
    contract = validate_contract(manifest, coverage)
    results: list[dict[str, Any]] = []
    if contract["ok"] and compiler.is_file():
        for case in manifest["cases"]:
            completed = subprocess.run(
                [str(compiler), "check", str(ROOT / case["source"])],
                cwd=ROOT,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                check=False,
            )
            combined = completed.stdout + completed.stderr
            passed, reasons = compare(case, completed.returncode, combined)
            results.append(
                {
                    "id": case["id"],
                    "clause": case["clause"],
                    "kind": case["kind"],
                    "passed": passed,
                    "actual_exit": completed.returncode,
                    "reasons": reasons,
                    "observed_marker": case["expected_contains"] in combined,
                }
            )
    elif not compiler.is_file():
        contract["ok"] = False
        contract["problems"].append(f"compiler not found: {compiler}")

    self_test = {"requested": args.self_test, "passed": None}
    if args.self_test and results:
        mutated = copy.deepcopy(manifest["cases"][0])
        mutated["expected_exit"] = 1 if mutated["expected_exit"] == 0 else 0
        detected, _ = compare(mutated, results[0]["actual_exit"], mutated["expected_contains"])
        self_test["passed"] = not detected

    passed_count = sum(1 for item in results if item["passed"])
    overall = (
        contract["ok"]
        and passed_count == len(manifest["cases"])
        and (not args.self_test or self_test["passed"] is True)
    )
    report = {
        "schema_version": 1,
        "task": "MAT-003",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "baseline_commit": manifest["baseline_commit"],
        "compiler": str(compiler),
        "compiler_sha256": sha256(compiler) if compiler.is_file() else None,
        "oracle": "conformance/normative/manifest.json",
        "oracle_independent_of_compiler": True,
        "contract": contract,
        "executed_cases": len(results),
        "passed_cases": passed_count,
        "failed_cases": len(results) - passed_count,
        "planned_core_cases": len(manifest.get("planned_core_cases", [])),
        "planned_core_cases_executed": 0,
        "self_test": self_test,
        "overall_pass": overall,
        "results": results,
    }
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        output = pathlib.Path(args.output)
        if not output.is_absolute():
            output = ROOT / output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if overall else 1


if __name__ == "__main__":
    sys.exit(main())
