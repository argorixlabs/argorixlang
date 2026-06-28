import hashlib
import importlib.util
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "paper" / "data"
FIGURES = {
    "architecture.pdf", "request-sequence.pdf", "decision-state-machine.pdf",
    "session-outcomes.pdf", "policy-heatmap.pdf", "evidence-chain.pdf",
    "trust-relationships.pdf", "threat-mitigation.pdf", "evolution-timeline.pdf",
    "sovereign-discovery.pdf", "artifact-schema.pdf", "claim-boundaries.pdf",
}
TABLES = {
    "dataset-inventory.tex", "language-constructs.tex", "runtime-controls.tex",
    "empirical-results.tex", "threat-mapping.tex", "related-work.tex",
    "claim-boundaries.tex",
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


def test_render_exact_latex_inventory_and_escape(tmp_path):
    out = tmp_path / "tables"
    run("render_tables.py", out)
    assert {p.name for p in out.iterdir()} == TABLES
    for tex in out.iterdir():
        text = tex.read_text(encoding="utf-8")
        assert "\\begin{tabular}" in text
        assert "\\end{tabular}" in text
        assert len(text) > 150
    claims = (out / "claim-boundaries.tex").read_text(encoding="utf-8")
    assert all(label in claims for label in ("Implemented", "Declarative", "Proposed", "Not claimed"))
    related = (out / "related-work.tex").read_text(encoding="utf-8")
    assert all(name in related for name in ("Project NANDA", "ATrust", "DCP-AI"))


def test_tables_are_deterministic_and_quantitative_values_are_derived(tmp_path):
    first, second = tmp_path / "a", tmp_path / "b"
    run("render_tables.py", first)
    run("render_tables.py", second)
    assert digests(first) == digests(second)
    empirical = (first / "empirical-results.tex").read_text(encoding="utf-8")
    assert "27" in empirical
    assert "6" in empirical


def test_latex_escape_covers_every_special_character():
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
        "architecture.pdf": [
            "Argorix source", "Parser + semantics", "Typed IR + bytecode",
            "Fail-closed VM", "Trace + ledger", "Evidence + reports",
        ],
        "artifact-schema.pdf": [
            "session.argx", "session.argbc.json", "session.trace.json",
            "session.security.json", "session.evidence.json",
        ],
        "claim-boundaries.pdf": [
            "Implemented", "Declarative", "Proposed", "Not claimed",
        ],
        "evolution-timeline.pdf": [
            "Core runtime", "Provider contracts", "Evidence + governance",
            "Operational federation",
        ],
    }
    for name, labels in expected.items():
        page = fitz.open(out / name)[0]
        words = page.get_text("words")
        text_tokens = [word[4] for word in words]
        label_boxes = []
        for label in labels:
            tokens = label.split()
            starts = [
                i for i in range(len(words) - len(tokens) + 1)
                if text_tokens[i:i + len(tokens)] == tokens
            ]
            assert len(starts) == 1, (name, label, text_tokens)
            group = words[starts[0]:starts[0] + len(tokens)]
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


def test_policy_heatmap_reserves_space_for_colorbar(tmp_path):
    out = tmp_path / "figures"
    run("generate_figures.py", out)
    page, blocks = _pdf_blocks(out / "policy-heatmap.pdf")
    title = next(box for box, text in blocks if "Policy and evidence" in text)
    tick_boxes = [box for box, text in blocks if text in {"2000", "3000", "4000", "5000", "6000", "7000"}]
    assert tick_boxes
    assert title[2] + 12 < min(box[0] for box in tick_boxes)
