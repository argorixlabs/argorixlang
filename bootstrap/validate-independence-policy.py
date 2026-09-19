#!/usr/bin/env python3
"""Validate the ESP-003 policy and candidate independence manifests."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import json
import pathlib
import re
import sys
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "bootstrap" / "independence-policy.json"


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_policy(policy: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if policy.get("schema_version") != 1:
        errors.append("unsupported policy schema")
    if set(policy.get("supported_profiles", [])) != {
        "ubuntu-24.04-x86_64",
        "windows-11-x86_64",
    }:
        errors.append("both required platform profiles must be present")
    essential = policy.get("essential_components", [])
    if len(essential) != 6 or len(set(essential)) != 6:
        errors.append("essential component set must contain six unique entries")
    stages = {item.get("id"): item for item in policy.get("stages", [])}
    if set(stages) != {"B0", "B1", "B2", "N1", "N2", "R1"}:
        errors.append("bootstrap stage set is incomplete")
    if stages.get("R1", {}).get("rust_allowed") is not False:
        errors.append("R1 must prohibit Rust")
    if stages.get("R1", {}).get("c_compiler_allowed") is not False:
        errors.append("R1 must prohibit the transitional C compiler")
    canaries = set(policy.get("sensor_canaries", []))
    required_canaries = {
        "rust_command",
        "rust_file",
        "rust_static_library",
        "rust_service",
        "backend_emits_rust",
        "cargo_cache",
        "c_compiler_final",
    }
    if not required_canaries.issubset(canaries):
        errors.append("sensor canary set is incomplete")
    return errors


def validate_candidate(candidate: dict[str, Any], policy: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    for field in policy["required_candidate_fields"]:
        if field not in candidate:
            errors.append(f"missing candidate field: {field}")
    if errors:
        return errors

    if candidate["profile"] not in policy["supported_profiles"]:
        errors.append("unsupported profile")
    stages = {item["id"]: item for item in policy["stages"]}
    stage = stages.get(candidate["stage"])
    if not stage:
        errors.append("unknown stage")
        stage = {}
    if candidate["stage"] == policy["final_stage"]:
        if stage.get("rust_allowed") or stage.get("c_compiler_allowed"):
            errors.append("final stage policy is not fail-closed")

    components = {item.get("name"): item for item in candidate["components"]}
    missing_components = set(policy["essential_components"]) - set(components)
    if missing_components:
        errors.append(f"missing essential components: {sorted(missing_components)}")
    for name in policy["essential_components"]:
        item = components.get(name, {})
        if item.get("implementation_language") != policy["required_source_language"]:
            errors.append(f"component {name} is not implemented in Argorix")

    prohibited_suffixes = tuple(value.lower() for value in policy["prohibited_file_suffixes"])
    for source in candidate["sources"]:
        path = str(source.get("path", ""))
        provenance = str(source.get("provenance", "")).lower()
        if path.lower().endswith(prohibited_suffixes) or "rust" in provenance:
            errors.append(f"Rust source/provenance prohibited: {path}")
        if not source.get("sha256"):
            errors.append(f"source without digest: {path}")

    allowed = set(policy["allowed_dependency_kinds"])
    transitional = set(policy["transitional_only_dependency_kinds"])
    prohibited = set(policy["prohibited_dependency_kinds"])
    dependency_kinds: set[str] = set()
    for dependency in candidate["dependencies"]:
        kind = dependency.get("kind")
        dependency_kinds.add(str(kind))
        missing = [field for field in policy["required_dependency_metadata"] if not dependency.get(field)]
        if missing:
            errors.append(f"dependency {dependency.get('name')} lacks metadata: {missing}")
        if kind in prohibited or (candidate["stage"] == "R1" and kind in transitional):
            errors.append(f"prohibited dependency kind at {candidate['stage']}: {kind}")
        elif kind not in allowed and kind not in transitional:
            errors.append(f"unknown dependency kind: {kind}")
        provenance = str(dependency.get("provenance", "")).lower()
        if dependency.get("contains_rust") is True or provenance.startswith("rust") or "compiled-from-rust" in provenance:
            errors.append(f"Rust dependency provenance prohibited: {dependency.get('name')}")
    if "system_linker" not in dependency_kinds:
        errors.append("system linker must be explicitly inventoried")

    prohibited_commands = set(policy["prohibited_commands"])
    for command in candidate["commands"]:
        tokens = re.findall(r"[A-Za-z0-9_.+-]+", str(command).lower())
        if prohibited_commands.intersection(tokens):
            errors.append(f"prohibited Rust command: {command}")
        if candidate["stage"] == "R1" and {"cc", "gcc", "clang", "cl.exe"}.intersection(tokens):
            errors.append(f"C compiler prohibited in final stage: {command}")
        if ".cargo" in str(command).lower() or "cargo_home" in str(command).lower():
            errors.append(f"Cargo cache access prohibited: {command}")

    expected_output = {
        "ubuntu-24.04-x86_64": "elf64-x86_64",
        "windows-11-x86_64": "pe-coff-x86_64",
    }.get(candidate["profile"])
    for output in candidate["backend_outputs"]:
        kind = str(output.get("kind", ""))
        if kind in {"rust-source", "rust", "rs"} or str(output.get("path", "")).lower().endswith(".rs"):
            errors.append("backend emits Rust")
    if expected_output and expected_output not in {item.get("kind") for item in candidate["backend_outputs"]}:
        errors.append(f"missing native backend output {expected_output}")

    for service in candidate["services"]:
        if str(service.get("implementation_language", "")).lower() == "rust":
            errors.append(f"Rust service prohibited: {service.get('name')}")
        if service.get("essential_phase"):
            errors.append(f"essential phase may not be delegated to a service: {service.get('name')}")

    for shim in candidate["shims"]:
        if not shim.get("symbols") or not shim.get("purpose") or not shim.get("owner"):
            errors.append(f"shim is not bounded/inventoried: {shim.get('name')}")
        if str(shim.get("implementation_language", "")).lower() == "rust":
            errors.append(f"Rust shim prohibited: {shim.get('name')}")
    return errors


def clean_candidate() -> dict[str, Any]:
    digest = "0" * 64
    return {
        "profile": "ubuntu-24.04-x86_64",
        "stage": "R1",
        "components": [
            {"name": name, "implementation_language": "argorix"}
            for name in ["compiler", "runtime", "verifier", "conformance", "signer", "package_manager"]
        ],
        "sources": [{"path": "compiler/main.argx", "sha256": digest, "provenance": "argorix-source"}],
        "dependencies": [{
            "name": "system-ld",
            "kind": "system_linker",
            "version": "pinned-by-candidate",
            "sha256": digest,
            "license": "system-component",
            "purpose": "link Argorix-emitted ELF objects",
            "owner": "release-engineering",
            "provenance": "system-non-rust"
        }],
        "commands": ["argorix build compiler/main.argx", "system-ld @objects.rsp"],
        "backend_outputs": [{"kind": "elf64-x86_64", "path": "out/argorix"}],
        "services": [],
        "shims": [{
            "name": "linux-host",
            "implementation_language": "c",
            "symbols": ["argx_write"],
            "purpose": "bounded OS ABI",
            "owner": "runtime-maintainer"
        }]
    }


def self_tests(policy: dict[str, Any]) -> list[dict[str, Any]]:
    cases: list[tuple[str, dict[str, Any], str | None]] = []
    cases.append(("accept-clean-native", clean_candidate(), None))
    rust_wrapper = clean_candidate()
    rust_wrapper["dependencies"].append({
        "name":"hidden-wrapper", "kind":"wrapper_to_rust", "version":"1", "sha256":"0"*64,
        "license":"test", "purpose":"negative control", "owner":"test"
    })
    cases.append(("reject-wrapper-to-rust", rust_wrapper, "prohibited dependency kind"))
    rust_static = clean_candidate()
    rust_static["dependencies"].append({
        "name":"libhidden.a", "kind":"audited_non_rust_library", "version":"1", "sha256":"0"*64,
        "license":"test", "purpose":"negative control", "owner":"test", "provenance":"compiled-from-rust", "contains_rust":True
    })
    cases.append(("reject-static-rust-provenance", rust_static, "Rust dependency provenance"))
    rust_service = clean_candidate()
    rust_service["services"].append({"name":"compiler-rpc", "implementation_language":"rust", "essential_phase":True})
    cases.append(("reject-rust-service", rust_service, "Rust service prohibited"))
    emits_rust = clean_candidate()
    emits_rust["backend_outputs"] = [{"kind":"rust-source", "path":"out/main.rs"}]
    cases.append(("reject-backend-emits-rust", emits_rust, "backend emits Rust"))
    cargo_cache = clean_candidate()
    cargo_cache["commands"].append("copy %CARGO_HOME%/registry/cache output")
    cases.append(("reject-cargo-cache", cargo_cache, "Cargo cache access prohibited"))
    rust_command = clean_candidate()
    rust_command["commands"].append("rustc hidden.rs -o hidden")
    cases.append(("reject-rust-command", rust_command, "prohibited Rust command"))
    rust_source = clean_candidate()
    rust_source["sources"].append({"path":"hidden.rs", "sha256":"0"*64, "provenance":"rust-source"})
    cases.append(("reject-rust-source", rust_source, "Rust source/provenance prohibited"))
    c_final = clean_candidate()
    c_final["dependencies"].append({
        "name":"clang", "kind":"c_compiler", "version":"1", "sha256":"0"*64,
        "license":"test", "purpose":"negative control", "owner":"test"
    })
    cases.append(("reject-c-compiler-in-r1", c_final, "prohibited dependency kind"))

    results: list[dict[str, Any]] = []
    for case_id, candidate, marker in cases:
        errors = validate_candidate(candidate, policy)
        passed = not errors if marker is None else any(marker in error for error in errors)
        results.append({"id": case_id, "passed": passed, "expected_marker": marker, "errors": errors})
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--output")
    args = parser.parse_args()
    policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    policy_errors = validate_policy(policy)
    candidate_errors: list[str] | None = None
    if args.candidate:
        candidate = json.loads(pathlib.Path(args.candidate).read_text(encoding="utf-8"))
        candidate_errors = validate_candidate(candidate, policy)
    tests = self_tests(policy) if args.self_test else []
    overall = not policy_errors and candidate_errors in (None, []) and all(item["passed"] for item in tests)
    report = {
        "schema_version": 1,
        "task": "ESP-003",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "policy": "bootstrap/independence-policy.json",
        "policy_sha256": sha256(POLICY_PATH),
        "policy_errors": policy_errors,
        "candidate_errors": candidate_errors,
        "self_tests_executed": len(tests),
        "self_tests_passed": sum(1 for item in tests if item["passed"]),
        "overall_pass": overall,
        "results": tests,
        "limitations": [
            "This validates the architecture contract and synthetic detector controls, not a future R1 candidate.",
            "Binary/process/network/SBOM sensors are required later by ESP-024."
        ]
    }
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        output = pathlib.Path(args.output)
        if not output.is_absolute():
            output = ROOT / output
        output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if overall else 1


if __name__ == "__main__":
    sys.exit(main())
