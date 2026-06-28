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
    assert "7 tables" in result.stdout
    assert "all figure/table labels referenced in prose" in result.stdout
