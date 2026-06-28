import json
from pathlib import Path


PAPER = Path(__file__).resolve().parents[1]


def test_build_script_pins_official_engine_and_checksum():
    script = (PAPER / "scripts/build_paper.ps1").read_text(encoding="utf-8")
    assert '$TectonicVersion = "0.16.9"' in script
    assert "github.com/tectonic-typesetting/tectonic/releases/download/" in script
    assert "131a24604785a9600989a3d91225f597df52ac06f00aeffe86fd529f99ee5cdd" in script
    assert "Get-FileHash" in script
    assert "SOURCE_DATE_EPOCH" in script


def test_makefile_exposes_reproducibility_targets():
    makefile = (PAPER / "Makefile").read_text(encoding="utf-8")
    for target in ("analyze", "verify", "figures", "tables", "paper", "test", "clean", "all"):
        assert f"{target}:" in makefile


def test_final_qa_schema_when_artifact_exists():
    path = PAPER / "data/final-qa.json"
    if not path.exists():
        return
    qa = json.loads(path.read_text(encoding="utf-8"))
    assert qa["dataset"] == {"total": 33, "complete": 27, "source_only": 6}
    assert qa["prompt_traces"] == {"evaluated": 27, "detected": 0}
    assert qa["verification"] == {"passed": 27, "total": 27}
    assert qa["figure_count"] == 12
    assert qa["table_count"] == 7
