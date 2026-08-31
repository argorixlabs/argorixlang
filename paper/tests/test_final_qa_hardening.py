import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import fitz
import pytest


PAPER = Path(__file__).resolve().parents[1]


def load(name):
    scripts = str(PAPER / "scripts")
    if scripts not in sys.path:
        sys.path.insert(0, scripts)
    path = PAPER / "scripts" / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def test_adversarial_failed_verification_is_rejected(tmp_path):
    qa = load("qa_pdf")
    data = tmp_path / "data"
    data.mkdir()
    (data / "runtime_summary.json").write_text(json.dumps({
        "total_sessions": 2, "complete_sessions": 2, "incomplete_sessions": 0,
        "traces_inspected_for_prompt_content": 2, "traces_with_prompt_content": 0,
    }), encoding="utf-8")
    (data / "verification-results.json").write_text(json.dumps([
        {"verified": False, "exit_code": 1},
        {"verified": False, "exit_code": 2},
    ]), encoding="utf-8")
    (data / "sessions.csv").write_text(
        "request_id,complete\none,true\ntwo,true\n", encoding="utf-8"
    )
    with pytest.raises(ValueError, match="verification"):
        qa.collect_data_metrics(data)


def test_font_embedding_checks_font_programs_not_presence(tmp_path):
    qa = load("qa_pdf")
    embedded = fitz.open(PAPER / "argorixlang-preprint.pdf")
    assert qa.all_fonts_embedded(embedded)

    path = tmp_path / "builtin-font.pdf"
    doc = fitz.open()
    page = doc.new_page()
    page.insert_text((72, 72), "uses a PDF base font", fontname="helv")
    doc.save(path)
    doc.close()
    nonembedded = fitz.open(path)
    assert not qa.all_fonts_embedded(nonembedded)


def test_input_manifest_excludes_outputs_and_changes_for_inputs(tmp_path):
    metadata = load("build_metadata")
    repo = tmp_path / "repo"
    repo.mkdir()
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True)
    for relative, value in {
        "paper/main.tex": "main",
        "paper/data/runtime_summary.json": "{}",
        "paper/data/final-qa.json": "{}",
        "paper/argorixlang-preprint.pdf": "pdf",
        "paper/tmp/cache": "tmp",
    }.items():
        path = repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value, encoding="utf-8")
    before = metadata.input_manifest(repo)
    (repo / "paper/data/final-qa.json").write_text("changed", encoding="utf-8")
    (repo / "paper/argorixlang-preprint.pdf").write_text("changed", encoding="utf-8")
    (repo / "paper/tmp/cache").write_text("changed", encoding="utf-8")
    assert metadata.input_manifest(repo) == before
    (repo / "paper/main.tex").write_text("changed", encoding="utf-8")
    assert metadata.input_manifest(repo)[0] != before[0]


def test_atomic_write_preserves_existing_file_on_writer_failure(tmp_path):
    atomic = load("atomic_io")
    target = tmp_path / "result.txt"
    target.write_text("old", encoding="utf-8")

    def fail(temp):
        temp.write_text("partial", encoding="utf-8")
        raise RuntimeError("interrupted")

    with pytest.raises(RuntimeError, match="interrupted"):
        atomic.atomic_publish(target, fail)
    assert target.read_text(encoding="utf-8") == "old"
    assert not list(tmp_path.glob("*.tmp"))


def test_build_script_uses_unique_atomic_publication_paths():
    script = (PAPER / "scripts" / "build_paper.ps1").read_text(encoding="utf-8")
    assert "Copy-Item -LiteralPath $sourcePdf -Destination $temporaryPdf" in script
    assert "Move-Item -LiteralPath $temporaryPdf -Destination $FinalPdf -Force" in script
    assert "[guid]::NewGuid().ToString('N')" in script
    assert '"test-results.json.tmp"' not in script
    assert '$env:ARGORIX_PAPER_INPUT_ROOT = Resolve-InputRoot' in script
