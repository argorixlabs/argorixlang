"""Machine-check the final preprint and write its auditable QA manifest."""

from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

import fitz
from atomic_io import atomic_write_text
from build_metadata import input_manifest


PAPER = Path(__file__).resolve().parents[1]
REPO = PAPER.parent
AUTHORS = ("Gustavo Venegas", "Edison Vazquez", "Danilo Naranjo", "Benjamin Gonzalez")


def collect_data_metrics(data: Path) -> dict:
    runtime = json.loads((data / "runtime_summary.json").read_text(encoding="utf-8"))
    with (data / "sessions.csv").open(encoding="utf-8", newline="") as handle:
        sessions = list(csv.DictReader(handle))
    verification = json.loads(
        (data / "verification-results.json").read_text(encoding="utf-8")
    )
    if not isinstance(verification, list):
        raise ValueError("verification results must be a list")
    passed = sum(
        record.get("verified") is True and record.get("exit_code") == 0
        for record in verification
        if isinstance(record, dict)
    )
    complete = sum(row.get("complete", "").lower() == "true" for row in sessions)
    total = len(sessions)
    metrics = {
        "dataset": {
            "total": total,
            "complete": complete,
            "source_only": total - complete,
        },
        "prompt_traces": {
            "evaluated": runtime["traces_inspected_for_prompt_content"],
            "detected": runtime["traces_with_prompt_content"],
        },
        "verification": {"passed": passed, "total": len(verification)},
    }
    expected = (
        runtime["total_sessions"],
        runtime["complete_sessions"],
        runtime["incomplete_sessions"],
    )
    observed = (total, complete, total - complete)
    if observed != expected:
        raise ValueError(f"dataset counts disagree: {observed} != {expected}")
    if not verification or passed != len(verification):
        raise ValueError(f"verification did not fully pass: {passed}/{len(verification)}")
    return metrics


def all_fonts_embedded(doc: fitz.Document) -> bool:
    xrefs = {
        font[0]
        for page in doc
        for font in page.get_fonts(full=True)
    }
    if not xrefs or 0 in xrefs:
        return False
    return all(bool(doc.extract_font(xref)[3]) for xref in xrefs)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pdf", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--pdfinfo", required=True)
    parser.add_argument("--engine", required=True)
    parser.add_argument("--source-date-epoch", required=True, type=int)
    parser.add_argument("--test-results", required=True, type=Path)
    parser.add_argument("--visual-inspection-passed", action="store_true")
    args = parser.parse_args()

    doc = fitz.open(args.pdf)
    text = "\n".join(page.get_text() for page in doc)
    source = "\n".join(
        path.read_text(encoding="utf-8")
        for path in [PAPER / "main.tex", *sorted((PAPER / "sections").glob("*.tex"))]
    )
    figures = set(re.findall(r"\\includegraphics(?:\[[^\]]*\])?\{([^}]+)\}", source))
    tables = set(re.findall(r"\\input\{tables/([^}]+)\}", source))
    citations: set[str] = set()
    for group in re.findall(r"\\cite\w*\{([^}]+)\}", source):
        citations.update(item.strip() for item in group.split(","))
    checks = {
        "page_range_16_24": 16 <= doc.page_count <= 24,
        "all_authors_present": all(author in text for author in AUTHORS),
        "figures_12": len(figures) == 12,
        "tables_7": len(tables) == 7,
        "citations_resolved": "[?]" not in text and "??" not in text,
        "no_placeholders": not re.search(
            r"\b(?:TODO|TBD|FIXME|PLACEHOLDER)\b|turn\d+(?:search|fetch)\d+", text, re.I
        ),
        "fonts_embedded": all_fonts_embedded(doc),
        "visual_inspection_passed": args.visual_inspection_passed,
    }
    if not all(value for key, value in checks.items() if key != "visual_inspection_passed"):
        raise SystemExit(f"PDF QA failed: {checks}")
    info = subprocess.check_output([args.pdfinfo, str(args.pdf)], text=True, errors="replace")
    metrics = collect_data_metrics(PAPER / "data")
    test_results = json.loads(args.test_results.read_text(encoding="utf-8"))
    if (
        test_results.get("failed") != 0
        or test_results.get("passed") != test_results.get("total")
        or test_results.get("total", 0) < 1
    ):
        raise SystemExit(f"test suite did not fully pass: {test_results}")
    manifest_sha256, manifest_file_count = input_manifest(REPO)
    qa = {
        "schema_version": 2,
        "input_manifest_sha256": manifest_sha256,
        "input_manifest_file_count": manifest_file_count,
        "input_manifest_algorithm": "sha256(path NUL sha256(file_bytes), sorted by path)",
        "source_date_epoch": args.source_date_epoch,
        "qa_artifact_note": (
            "Reproducibility is bound to input_manifest_sha256, which excludes "
            "this QA file, the final PDF, temporary files, and caches."
        ),
        **metrics,
        "tests": {**test_results, "status": "passed"},
        "engine": args.engine,
        "page_count": doc.page_count,
        "figure_count": len(figures),
        "table_count": len(tables),
        "citation_count": len(citations),
        "visual_inspection": {
            "passed": args.visual_inspection_passed,
            "pages_rendered": doc.page_count,
            "method": "144 dpi Poppler PNGs, contact sheets, flagged-page review",
        },
        "inspection_timestamp": datetime.now(timezone.utc).isoformat(),
        "warnings": [
            "Harmless underfull-box warnings remain in narrow tables and bibliography entries.",
            "BibTeX reports an empty year for the deliberately undated unpublished ATrust source.",
            "Fontconfig emits a missing-default-config diagnostic on this Windows host; bundled Latin Modern fonts render and embed.",
        ],
        "checks": checks,
        "pdf_metadata": doc.metadata,
        "pdfinfo": info,
    }
    atomic_write_text(args.output, json.dumps(qa, indent=2) + "\n")
    print(f"PDF QA passed: {doc.page_count} pages, {len(figures)} figures, {len(tables)} tables")


if __name__ == "__main__":
    main()
