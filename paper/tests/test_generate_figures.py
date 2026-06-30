import hashlib
import importlib.util
import csv
import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "paper" / "data"
FIGURES = {
    "system-pipeline.pdf", "provider-boundary.pdf", "policy-lattice-flow.pdf",
    "evidence-scope.pdf", "claim-boundaries.pdf", "controlled-matrix-outcomes.pdf",
    "threat-control-mapping.pdf",
}
TABLES = {
    "ablation-study.tex", "baseline-comparison.tex",
    "controlled-evaluation-matrix.tex", "controlled-matrix-summary.tex",
    "dataset-inventory.tex", "language-constructs.tex", "runtime-controls.tex",
    "empirical-results.tex", "threat-mapping.tex", "related-work.tex",
    "claim-boundaries.tex", "limitations-future-assurance.tex",
    "matrix-provider-boundary.tex", "matrix-tamper-results.tex",
    "passport-jurisdiction-coverage.tex", "policy-lattice.tex",
    "policy-lattice-outcomes.tex",
}


def run(script: str, output: Path) -> None:
    subprocess.run(
        [sys.executable, str(ROOT / "paper" / "scripts" / script),
         "--data", str(DATA), "--output", str(output)],
        check=True, cwd=ROOT,
    )


def digests(directory: Path) -> dict[str, str]:
    return {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(directory.iterdir())
    }


def test_generate_exact_vector_pdf_inventory(tmp_path):
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    assert {p.name for p in out.iterdir()} == FIGURES
    for pdf in out.iterdir():
        payload = pdf.read_bytes()
        assert payload.startswith(b"%PDF-")
        assert len(payload) > 2_000
        assert b"/Type /Font" in payload


def test_figures_are_deterministic(tmp_path):
    first, second = tmp_path / "a", tmp_path / "b"
    run("generate_figures.py", first)
    run("generate_figures.py", second)
    assert digests(first) == digests(second)


def test_evidence_figure_matches_verifier_scope(tmp_path):
    import fitz
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    text = " ".join(fitz.open(out / "evidence-scope.pdf")[0].get_text().split())
    for required in (
        "EvidenceBundle verification scope",
        "historical v0.1 bundle verification",
        "session.argx source",
        "bytecode",
        "trace",
        "source_digest",
        "mismatch -> VERIFIER_FAIL",
    ):
        assert required in text
    assert "Tamper-evident evidence chain" not in text


def test_render_exact_latex_inventory_and_escape(tmp_path):
    out = tmp_path / "tables"
    run("render_tables.py", out)
    assert {p.name for p in out.iterdir()} == TABLES
    for tex in out.iterdir():
        text = tex.read_text(encoding="utf-8")
        assert "\\begin{tabularx}" in text
        assert "\\end{tabularx}" in text
        assert len(text) > 150
    claims = (out / "claim-boundaries.tex").read_text(encoding="utf-8")
    assert all(label in claims for label in ("Implemented", "Declarative", "Proposed", "Not claimed"))
    related = (out / "related-work.tex").read_text(encoding="utf-8")
    assert all(name in related for name in ("Project NANDA", "ATrust", "DCP-AI", "SPIFFE"))
    controlled = (out / "controlled-evaluation-matrix.tex").read_text(encoding="utf-8")
    assert "Generated in controlled matrix" in controlled
    ablation = (out / "ablation-study.tex").read_text(encoding="utf-8")
    assert "Do not claim until measured" in ablation
    coverage = (out / "passport-jurisdiction-coverage.tex").read_text(encoding="utf-8")
    assert "latin\\_america" in coverage
    summary = (out / "controlled-matrix-summary.tex").read_text(encoding="utf-8")
    assert "Countries tested & 31" in summary


def test_tables_are_deterministic_and_quantitative_values_are_derived(tmp_path):
    first, second = tmp_path / "a", tmp_path / "b"
    run("render_tables.py", first)
    run("render_tables.py", second)
    assert digests(first) == digests(second)
    empirical = (first / "empirical-results.tex").read_text(encoding="utf-8")
    assert "27" in empirical
    assert "6" in empirical
    assert "Prompt-content traces inspected / present & 27 / 0 &" in empirical


def test_latex_escape_covers_every_special_character():
    sys.path.insert(0, str(ROOT / "paper" / "scripts"))
    path = ROOT / "paper" / "scripts" / "render_tables.py"
    spec = importlib.util.spec_from_file_location("render_tables", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    escaped = module.esc(r"\&%$#_{}~^")
    assert escaped == (
        r"\textbackslash{}\&\%\$\#\_\{\}\textasciitilde{}\textasciicircum{}"
    )


def _pdf_blocks(path):
    import fitz
    doc = fitz.open(path)
    page = doc[0]
    blocks = [
        (tuple(block[:4]), " ".join(block[4].split()))
        for block in page.get_text("blocks")
        if block[4].strip()
    ]
    return page.rect, blocks


def test_every_pdf_text_block_is_inside_page(tmp_path):
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    for pdf in out.glob("*.pdf"):
        page, blocks = _pdf_blocks(pdf)
        assert blocks, pdf.name
        for (x0, y0, x1, y1), text in blocks:
            assert x0 >= 0 and y0 >= 0 and x1 <= page.width and y1 <= page.height, (
                pdf.name, text, (x0, y0, x1, y1), page
            )


def test_flow_node_labels_have_separate_nonoverlapping_text_blocks(tmp_path):
    import fitz
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    expected = {
        "system-pipeline.pdf": [
            "Argorix source", "Parser + semantics", "Typed IR / bytecode",
            "Fail-closed VM", "Trace / ledger", "Evidence / reports",
        ],
        "provider-boundary.pdf": [
            "Caller", "Compiler validation", "Policy lattice", "VM",
            "Executable registry", "Evidence",
        ],
        "claim-boundaries.pdf": [
            "Implemented", "Declarative", "Proposed", "Not claimed",
        ],
    }
    for name, labels in expected.items():
        page = fitz.open(out / name)[0]
        words = page.get_text("words")
        text_tokens = [word[4] for word in words]
        label_boxes = []
        for label in labels:
            tokens = label.split()
            normalized = "".join(tokens)
            matches = []
            for start in range(len(words)):
                for count in range(1, 5):
                    group = words[start:start + count]
                    if "".join(word[4] for word in group) == normalized:
                        matches.append(group)
            assert len(matches) == 1, (name, label, text_tokens)
            group = matches[0]
            label_boxes.append((
                min(w[0] for w in group), min(w[1] for w in group),
                max(w[2] for w in group), max(w[3] for w in group),
            ))
        ordered = sorted(label_boxes)
        for left, right in zip(ordered, ordered[1:]):
            assert left[2] + 4 <= right[0], (name, left, right)


def test_claim_boundary_table_has_four_status_columns(tmp_path):
    out = tmp_path / "tables"
    run("render_tables.py", out)
    lines = (out / "claim-boundaries.tex").read_text(encoding="utf-8").splitlines()
    header = next(line for line in lines if "Implemented" in line)
    assert header == (
        r"Concept & Implemented & Declarative & Proposed & Not claimed \\"
    )
    text = "\n".join(lines)
    assert "local ans\\_name binding/metadata" in text
    assert "local catalog" not in text


def test_controlled_matrix_outcomes_are_sober_bars(tmp_path):
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    text = " ".join(__import__("fitz").open(out / "controlled-matrix-outcomes.pdf")[0].get_text().split())
    for required in ("PASS", "DENY", "REVIEW", "UNKNOWN_RULE", "ERROR", "VERIFIER_FAIL", "57 deterministic controlled cases"):
        assert required in text
    assert "Policy and evidence event volume" not in text


def test_figures_are_legible_at_seven_inch_placement(tmp_path):
    import fitz
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    for pdf in out.glob("*.pdf"):
        page = fitz.open(pdf)[0]
        assert page.rect.width <= 7.2 * 72, (pdf.name, page.rect.width)
        spans = [
            span
            for block in page.get_text("dict")["blocks"]
            for line in block.get("lines", [])
            for span in line["spans"]
            if span["text"].strip()
        ]
        assert min(span["size"] for span in spans) >= 7.0, pdf.name
        for span in spans:
            if span["text"].strip() not in {"PROPOSED /", "NOT IMPLEMENTED"}:
                assert span["size"] >= 8.0, (pdf.name, span["text"], span["size"])


def test_declared_python_dependencies_match_runtime():
    import fitz
    import matplotlib
    import pytest
    requirements = (ROOT / "paper" / "requirements.txt").read_text(encoding="utf-8")
    assert "matplotlib==3.10.8" in requirements
    assert "pytest==9.0.2" in requirements
    assert "PyMuPDF==1.27.1" in requirements
    assert "bibtexparser==1.4.4" in requirements
    assert matplotlib.__version__ == "3.10.8"
    assert pytest.__version__ == "9.0.2"
    assert fitz.__doc__ and "1.27.1" in fitz.__doc__


def test_mutated_normalized_data_recalculates_tables_and_empirical_figures(tmp_path):
    import fitz
    data = tmp_path / "data"
    shutil.copytree(DATA, data)
    summary = json.loads((data / "runtime_summary.json").read_text(encoding="utf-8"))
    summary["complete_sessions"] = 2
    summary["incomplete_sessions"] = 1
    (data / "runtime_summary.json").write_text(json.dumps(summary), encoding="utf-8")

    with (DATA / "sessions.csv").open(encoding="utf-8", newline="") as source:
        original = list(csv.DictReader(source))
        fields = source.readline() if False else original[0].keys()
    rows = [dict(original[0]), dict(original[1]), dict(original[2])]
    for row, complete, violations, ledger in zip(
        rows, ("true", "true", "false"), ("7", "11", "0"), ("13", "17", "0")
    ):
        row["complete"] = complete
        row["policy_violation_count"] = violations
        row["ledger_events_total"] = ledger
    with (data / "sessions.csv").open("w", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(target, fieldnames=list(fields))
        writer.writeheader()
        writer.writerows(rows)
    with (data / "event_counts.csv").open("w", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(target, fieldnames=["request_id", "event_kind", "count"])
        writer.writeheader()
        writer.writerows([
            {"request_id": "a", "event_kind": "One", "count": 19},
            {"request_id": "b", "event_kind": "Two", "count": 23},
        ])
    (data / "verification-results.json").write_text(json.dumps([
        {"verified": True}, {"verified": False}, {"verified": True},
    ]), encoding="utf-8")

    figures, tables = tmp_path / "figures", tmp_path / "tables"
    subprocess.run([sys.executable, str(ROOT / "paper/scripts/generate_figures.py"),
                    "--data", str(data), "--output", str(figures)], check=True, cwd=ROOT)
    subprocess.run([sys.executable, str(ROOT / "paper/scripts/render_tables.py"),
                    "--data", str(data), "--output", str(tables)], check=True, cwd=ROOT)
    empirical = (tables / "empirical-results.tex").read_text(encoding="utf-8")
    for value in ("2", "1", "3", "18", "42"):
        assert f"& {value} &" in empirical
    assert "Verified evidence bundles & 2 / 3 &" in empirical
    matrix = " ".join(fitz.open(figures / "controlled-matrix-outcomes.pdf")[0].get_text().split())
    assert "57 deterministic controlled cases" in matrix


def test_tables_use_bounded_tabularx_widths(tmp_path):
    out = tmp_path / "tables"
    run("render_tables.py", out)
    for tex in out.glob("*.tex"):
        text = tex.read_text(encoding="utf-8")
        assert "\\begin{tabularx}" in text
        assert "\\end{tabularx}" in text
        expected = r"\textwidth" if tex.name == "claim-boundaries.tex" else r"\linewidth"
        assert f"{{{expected}}}" in text
