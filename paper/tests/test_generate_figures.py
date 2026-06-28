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
