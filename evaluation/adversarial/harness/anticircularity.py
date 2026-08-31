"""Harness-validity test.

Two independent properties are checked:

1. `collect` cannot read the oracle.  Enforced statically (no reference to the
   file anywhere in the collection path) and dynamically (a guard turns the
   read into a hard error, exercised here on purpose).
2. `collect` cannot fabricate a correct result without executing Argorix.  A
   copy of the binary directory with one executable removed must make the
   campaign fail rather than produce outcomes.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

from util import BINARIES, EVAL_ROOT, REPO_ROOT, write_json  # noqa: E402

COLLECTION_PATH = ["collect.py", "util.py", "mutate.py", "canaries.py", "stats.py"]


def static_check() -> dict[str, Any]:
    """No module on the collection path may name the oracle file."""
    offenders = []
    for name in COLLECTION_PATH:
        path = Path(__file__).parent / name
        text = path.read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), start=1):
            if "oracle.json" not in line:
                continue
            # The guard list and its docstring are the only permitted mentions.
            if "FORBIDDEN_READS" in line or line.strip().startswith("#"):
                continue
            if "must not read" in line or "never read" in line:
                continue
            offenders.append({"file": name, "line": number, "text": line.strip()})
    return {
        "name": "collect_does_not_reference_oracle",
        "passed": not offenders,
        "offenders": offenders,
    }


def guard_check() -> dict[str, Any]:
    """The runtime guard must turn an oracle read into an error."""
    script = (
        "import sys;"
        f"sys.path.insert(0, {str(Path(__file__).parent)!r});"
        "import collect;"
        "collect.install_oracle_guard();"
        "\n"
        "try:\n"
        f"    open({str(EVAL_ROOT / 'oracle.json')!r}, 'r', encoding='utf-8').read()\n"
        "except PermissionError as error:\n"
        "    print('BLOCKED')\n"
        "else:\n"
        "    print('LEAKED')\n"
    )
    completed = subprocess.run(  # noqa: S603
        [sys.executable, "-c", script],
        capture_output=True,
        text=True,
        timeout=120,
    )
    blocked = "BLOCKED" in completed.stdout
    return {
        "name": "oracle_read_is_blocked_at_runtime",
        "passed": blocked,
        "stdout": completed.stdout.strip(),
        "stderr": completed.stderr.strip()[-400:],
    }


def missing_binary_check(bin_dir: Path, removed: str = "argorix-vm") -> dict[str, Any]:
    """Removing a production binary must make the campaign fail."""
    with tempfile.TemporaryDirectory(prefix="argorix-eval-nobin-") as temporary:
        crippled = Path(temporary) / "bin"
        crippled.mkdir(parents=True)
        for key, filename in BINARIES.items():
            source = bin_dir / filename
            if key == removed or not source.is_file():
                continue
            shutil.copy2(source, crippled / filename)
        completed = subprocess.run(  # noqa: S603
            [
                sys.executable,
                str(Path(__file__).parent / "collect.py"),
                "--cases",
                str(EVAL_ROOT / "cases.json"),
                "--bin-dir",
                str(crippled),
                "--out-dir",
                str(Path(temporary) / "results"),
                "--run-id",
                "anticircularity",
                "--family",
                "E1",
            ],
            capture_output=True,
            text=True,
            timeout=600,
        )
        produced_rows = (
            Path(temporary) / "results" / "raw" / "anticircularity" / "rows.jsonl"
        ).is_file()
    return {
        "name": "campaign_fails_without_the_binaries",
        "removed_binary": removed,
        "passed": completed.returncode != 0 and not produced_rows,
        "exit_code": completed.returncode,
        "produced_rows": produced_rows,
        "stderr_tail": completed.stderr.strip()[-400:],
    }


def run(bin_dir: Path) -> dict[str, Any]:
    checks = [static_check(), guard_check(), missing_binary_check(bin_dir)]
    passed = all(check["passed"] for check in checks)
    return {
        "passed": passed,
        "checks": checks,
        "summary": (
            "collect cannot read the oracle and cannot produce rows without the "
            "production binaries"
            if passed
            else "harness validity failed: "
            + ", ".join(check["name"] for check in checks if not check["passed"])
        ),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Harness validity / anti-circularity test")
    parser.add_argument("--bin-dir", default=str(REPO_ROOT / "target" / "release"))
    parser.add_argument("--out", default=None)
    args = parser.parse_args(argv)

    result = run(Path(args.bin_dir))
    print(json.dumps(result, indent=2))
    if args.out:
        write_json(Path(args.out), result)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
