"""Single reproduction entrypoint for the adversarial campaign.

    python evaluation/adversarial/run.py

Builds nothing by default: point `--bin-dir` at release binaries produced with

    cargo build --locked --release -p argorixc -p argorix-vm -p argorix-conformance

The run order is fixed: harness-validity test, collection, scoring, table
generation.  Collection and scoring are separate processes and the collector
cannot read `oracle.json`.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

HARNESS = Path(__file__).resolve().parent / "harness"
EVAL_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EVAL_ROOT.parents[1]


def step(title: str, argv: list[str]) -> int:
    print(f"\n=== {title} ===", flush=True)
    completed = subprocess.run(argv)  # noqa: S603
    return completed.returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run the full adversarial campaign")
    parser.add_argument("--bin-dir", default=str(REPO_ROOT / "target" / "release"))
    parser.add_argument("--run-id", default=None)
    parser.add_argument("--results-dir", default=str(EVAL_ROOT / "results"))
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--clean", action="store_true", help="delete previous results first")
    parser.add_argument("--skip-anticircularity", action="store_true")
    args = parser.parse_args(argv)

    run_id = args.run_id or datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    results_dir = Path(args.results_dir)
    if args.clean and results_dir.exists():
        shutil.rmtree(results_dir)
    raw_dir = results_dir / "raw" / run_id
    raw_dir.mkdir(parents=True, exist_ok=True)

    if not args.skip_anticircularity:
        code = step(
            "harness validity (anti-circularity)",
            [
                sys.executable,
                str(HARNESS / "anticircularity.py"),
                "--bin-dir",
                args.bin_dir,
                "--out",
                str(raw_dir / "anticircularity.json"),
            ],
        )
        if code != 0:
            print("harness validity failed: campaign aborted", file=sys.stderr)
            return code

    code = step(
        "collect",
        [
            sys.executable,
            str(HARNESS / "collect.py"),
            "--cases",
            str(EVAL_ROOT / "cases.json"),
            "--bin-dir",
            args.bin_dir,
            "--out-dir",
            str(results_dir),
            "--run-id",
            run_id,
            "--timeout",
            str(args.timeout),
        ],
    )
    if code != 0:
        return code

    code = step(
        "score",
        [
            sys.executable,
            str(HARNESS / "score.py"),
            "--run-id",
            run_id,
            "--results-dir",
            str(results_dir),
            "--oracle",
            str(EVAL_ROOT / "oracle.json"),
        ],
    )
    if code != 0:
        return code

    for out_dir in (
        EVAL_ROOT / "tables",
        REPO_ROOT / "paper" / "camera-ready" / "tables",
    ):
        code = step(
            f"render tables -> {out_dir}",
            [
                sys.executable,
                str(HARNESS / "render_tables.py"),
                "--summary",
                str(results_dir / "summary.json"),
                "--out-dir",
                str(out_dir),
                "--baseline",
                str(EVAL_ROOT / "baseline" / "prefix" / "summary.json"),
            ],
        )
        if code != 0:
            return code

    code = step(
        "render figure",
        [
            sys.executable,
            str(HARNESS / "render_figure.py"),
            "--summary",
            str(results_dir / "summary.json"),
            "--out",
            str(
                REPO_ROOT
                / "paper"
                / "camera-ready"
                / "figures"
                / "adversarial-boundary.pdf"
            ),
        ],
    )
    if code != 0:
        return code

    summary = json.loads((results_dir / "summary.json").read_text(encoding="utf-8"))
    print("\n=== campaign complete ===")
    print(f"run_id: {run_id}")
    print(f"rows:   {summary['rows_total']}")
    for name, gate in summary["gates"].items():
        print(f"gate {name}: {gate['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
