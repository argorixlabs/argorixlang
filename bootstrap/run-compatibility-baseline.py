"""Run the pinned ESP-002 conformance baseline from a clean checkout.

Only stage names/statuses are normalized. Diagnostics, paths and timing are
deliberately excluded; the independently maintained golden file is not derived
from the process being tested.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


HERE = Path(__file__).resolve().parent


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def command(argv: list[str], cwd: Path) -> str:
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def require(actual: object, expected: object, context: str) -> None:
    if actual != expected:
        raise ValueError(f"{context}: expected {expected!r}, observed {actual!r}")


def normalized(result: dict) -> list[dict]:
    return [
        {
            "id": case["id"],
            "stages": [f"{stage['stage']}={stage['status']}" for stage in case["stages"]],
        }
        for case in result["case_results"]
    ]


def validate_goldens(suite: str, cases: list[dict], goldens: list[dict]) -> int:
    by_id = {case["id"]: case for case in cases}
    require(len(by_id), len(cases), f"{suite} duplicate result IDs")
    selected = [fixture for fixture in goldens if fixture["suite"] == suite]
    for fixture in selected:
        actual = by_id.get(fixture["id"])
        if actual is None:
            raise ValueError(f"{suite} missing golden case {fixture['id']}")
        require(actual["stages"], fixture["stages"], f"{suite}/{fixture['id']}")
    return len(selected)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True, help="Clean checkout of pinned commit")
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, help="Optional generated report path, usually under target/")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    repo = args.repo.resolve()
    bin_dir = args.bin_dir.resolve()
    baseline = json.loads((HERE / "baseline.json").read_text(encoding="utf-8"))
    golden_path = HERE.parent / "tests" / "compatibility" / "golden.json"
    golden = json.loads(golden_path.read_text(encoding="utf-8"))
    require(sha256(golden_path), baseline["golden_sha256"], "golden hash")
    snapshot_spec = baseline["e0_snapshot_archive"]
    snapshot_archive = HERE.parent / snapshot_spec["path"]
    require(sha256(snapshot_archive), snapshot_spec["sha256"], "E0 snapshot archive hash")
    with zipfile.ZipFile(snapshot_archive) as archive:
        entries = [name for name in archive.namelist() if not name.endswith("/")]
    require(len(entries), snapshot_spec["files"], "E0 archive file count")
    if any(not name.startswith("generated/") or ".." in Path(name).parts for name in entries):
        raise ValueError("E0 archive has an unsafe entry path")
    request_dirs = {Path(name).parts[1] for name in entries if Path(name).parts[-1] == "session.argx"}
    complete_dirs = {Path(name).parts[1] for name in entries if Path(name).parts[-1] == "session.evidence.json"}
    require(len(request_dirs), snapshot_spec["request_directories"], "E0 request directory count")
    require(len(complete_dirs), snapshot_spec["complete_directories"], "E0 complete directory count")
    require(len(request_dirs - complete_dirs), snapshot_spec["source_only_directories"], "E0 source-only directory count")
    require(command(["git", "rev-parse", "HEAD"], repo), baseline["commit"], "commit")
    require(command(["git", "status", "--porcelain"], repo), "", "clean checkout")
    require(sha256(repo / "Cargo.lock"), baseline["cargo_lock_sha256"], "Cargo.lock hash")
    require(command(["rustc", "--version"], repo), baseline["rustc"], "rustc version")
    require(command(["cargo", "--version"], repo), baseline["cargo"], "cargo version")
    require(platform.system(), baseline["platform"], "host platform")

    executable = bin_dir / ("argorix-conformance.exe" if platform.system() == "Windows" else "argorix-conformance")
    if not executable.is_file():
        raise FileNotFoundError(executable)
    report = {
        "schema_version": 1,
        "commit": baseline["commit"],
        "platform": platform.platform(),
        "python": sys.version.split()[0],
        "rustc": baseline["rustc"],
        "cargo": baseline["cargo"],
        "cargo_lock_sha256": baseline["cargo_lock_sha256"],
        "conformance_binary_sha256": sha256(executable),
        "e0_snapshot_archive_sha256": snapshot_spec["sha256"],
        "normalization": golden["normalization"],
        "suites": [],
    }
    suite_files = {item.get("file", f"suite.{item['version']}.json") for item in baseline["suites"]}
    require(suite_files, {path.name for path in (repo / "conformance").glob("suite*.json")}, "suite file coverage")
    if {fixture["suite"] for fixture in golden["cases"]} - {item["version"] for item in baseline["suites"]}:
        raise ValueError("golden references a suite outside the pinned manifest")
    with tempfile.TemporaryDirectory(prefix="argorix-esp002-") as temp:
        for item in baseline["suites"]:
            version = item["version"]
            suite_path = repo / "conformance" / item.get("file", f"suite.{version}.json")
            require(sha256(suite_path), item["sha256"], f"{version} suite hash")
            source_cases = json.loads(suite_path.read_text(encoding="utf-8"))["cases"]
            require(len(source_cases), item["cases"], f"{version} source case count")
            runs = []
            for repetition in (1, 2):
                workdir = Path(temp) / f"{version}-{repetition}"
                process = subprocess.run(
                    [str(executable), "run", str(suite_path), "--workdir", str(workdir), "--json"],
                    cwd=repo, text=True, capture_output=True, check=False,
                )
                if item.get("expected_result") == "REJECTED":
                    if process.returncode == 0 or item["expected_diagnostic"] not in process.stderr:
                        raise ValueError(f"{version} rejection changed: {process.stderr[:300]}")
                    runs.append("REJECTED:" + item["expected_diagnostic"])
                    continue
                if process.returncode != 0:
                    raise ValueError(f"{version} runner failed: {process.stderr[:400]}")
                raw = process.stdout
                result = json.loads(raw)
                require(result["passed"], True, f"{version} run {repetition} passed")
                require(result["cases_total"], item["cases"], f"{version} case count")
                require(result["cases_passed"], item["cases"], f"{version} passed count")
                require(result["cases_failed"], 0, f"{version} failures")
                cases = normalized(result)
                require([case["id"] for case in cases], [case["id"] for case in source_cases], f"{version} case order")
                golden_count = validate_goldens(version, cases, golden["cases"])
                digest = hashlib.sha256(json.dumps(cases, sort_keys=True, separators=(",", ":")).encode()).hexdigest().upper()
                runs.append(digest)
            require(runs[0], runs[1], f"{version} offline rerun fingerprint")
            if item.get("expected_result") == "REJECTED":
                report["suites"].append({"version": version, "cases": item["cases"], "expected_rejection": True, "diagnostic": item["expected_diagnostic"], "rerun_equal": True})
            else:
                report["suites"].append({
                    "version": version,
                    "cases": item["cases"],
                    "passed": item["cases"],
                    "golden_cases": golden_count,
                    "normalized_sha256": runs[0],
                    "rerun_equal": True,
                })

    if args.self_test:
        injected = [dict(fixture) for fixture in golden["cases"]]
        injected[0]["stages"] = ["parse=impossible"]
        try:
            validate_goldens("v016", [{"id": golden["cases"][0]["id"], "stages": golden["cases"][0]["stages"]}], injected)
        except ValueError as error:
            if "v016/" not in str(error):
                raise
            report["negative_control"] = "PASS: altered expected stage rejected in memory"
        else:
            raise AssertionError("negative control accepted an altered golden")

    output = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    print(output, end="")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"ESP-002 baseline failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
