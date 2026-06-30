import subprocess
import sys
from pathlib import Path


def test_manuscript_static_contract() -> None:
    paper = Path(__file__).resolve().parents[1]
    result = subprocess.run(
        [sys.executable, str(paper / "scripts" / "check_manuscript.py")],
        cwd=paper.parent,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "12 figures" in result.stdout
    assert "12 tables" in result.stdout
    assert "all figure/table labels referenced in prose" in result.stdout


def test_readme_documents_complete_reproduction_pipeline() -> None:
    paper = Path(__file__).resolve().parents[1]
    readme = (paper / "README.md").read_text(encoding="utf-8")
    for fragment in (
        "python -m pip install -r paper/requirements.txt",
        "analyze_runtime.py --input",
        "--summary paper/data/runtime_summary.json",
        "--sessions paper/data/sessions.csv",
        "--events paper/data/event_counts.csv",
        "verify_runtime.ps1",
        "-InputRoot",
        "-OutputPath",
        "-CargoPath",
        "render_tables.py --data paper/data --output paper/tables",
        "generate_figures.py --data paper/data --output paper/figures",
        "python -m pytest paper/tests -q",
        "immutable input snapshot",
        "worktree",
    ):
        assert fragment in readme


def test_title_has_two_affiliations_with_author_markers() -> None:
    paper = Path(__file__).resolve().parents[1]
    main = (paper / "main.tex").read_text(encoding="utf-8")
    author_block = main.split(r"\author{", 1)[1].split("\n}\n", 1)[0]
    for name in (
        r"Gustavo Venegas\textsuperscript{1}",
        r"Edison Vazquez\textsuperscript{1}",
        r"Danilo Naranjo\textsuperscript{2}",
        r"Benjamin Gonzalez\textsuperscript{1}",
    ):
        assert name in author_block
    assert author_block.count("Chilean Chamber of Artificial Intelligence") == 1
    assert author_block.count("Ocular") == 1
