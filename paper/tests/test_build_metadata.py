import json
import os
import subprocess
from pathlib import Path


PAPER = Path(__file__).resolve().parents[1]
METADATA = PAPER / "scripts/build_metadata.py"


def git(repo: Path, *args: str, env: dict[str, str] | None = None) -> str:
    return subprocess.check_output(
        ["git", *args], cwd=repo, env=env, text=True, stderr=subprocess.STDOUT
    ).strip()


def commit(repo: Path, message: str, epoch: int) -> None:
    env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "Test",
        "GIT_AUTHOR_EMAIL": "test@example.invalid",
        "GIT_COMMITTER_NAME": "Test",
        "GIT_COMMITTER_EMAIL": "test@example.invalid",
        "GIT_AUTHOR_DATE": f"{epoch} +0000",
        "GIT_COMMITTER_DATE": f"{epoch} +0000",
    }
    git(repo, "add", ".", env=env)
    git(repo, "commit", "-m", message, env=env)


def test_build_script_pins_official_engine_and_checksum():
    script = (PAPER / "scripts/build_paper.ps1").read_text(encoding="utf-8")
    assert '$TectonicVersion = "0.16.9"' in script
    assert "github.com/tectonic-typesetting/tectonic/releases/download/" in script
    assert "131a24604785a9600989a3d91225f597df52ac06f00aeffe86fd529f99ee5cdd" in script
    assert "a0a9a5eaf1a940d9a615ad78d35225ca59420c7984576c6402fffb3e9fb05ceb" in script
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
    if qa.get("schema_version", 1) < 2:
        return
    assert qa["dataset"] == {"total": 33, "complete": 27, "source_only": 6}
    assert qa["prompt_traces"] == {"evaluated": 27, "detected": 0}
    assert qa["verification"] == {"passed": 27, "total": 27}
    assert qa["figure_count"] == 12
    assert qa["table_count"] == 12
    assert "source_commit" not in qa and "base" not in qa
    assert len(qa["input_manifest_sha256"]) == 64
    assert qa["input_manifest_file_count"] > 0
    assert qa["input_manifest_algorithm"].startswith("sha256")
    assert "reproducibility is bound" in qa["qa_artifact_note"].lower()
    assert qa["tests"]["passed"] == qa["tests"]["total"]
    assert qa["tests"]["failed"] == 0


def test_stable_epoch_ignores_build_tooling_and_generated_successor(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init")
    (repo / "paper").mkdir()
    (repo / "paper/main.tex").write_text("content", encoding="utf-8")
    commit(repo, "paper content", 1_700_000_000)
    (repo / "paper/scripts").mkdir()
    (repo / "paper/scripts/build_paper.ps1").write_text("tool", encoding="utf-8")
    (repo / "paper/argorixlang-preprint.pdf").write_bytes(b"pdf")
    (repo / "paper/data").mkdir()
    (repo / "paper/data/final-qa.json").write_text("{}", encoding="utf-8")
    commit(repo, "build successor", 1_800_000_000)

    epoch = subprocess.check_output(
        ["python", str(METADATA), "stable-epoch", "--repo", str(repo)], text=True
    ).strip()
    assert epoch == "1700000000"


def test_base_resolution_works_detached_without_local_main(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init")
    (repo / "one").write_text("1", encoding="utf-8")
    commit(repo, "one", 1_700_000_000)
    parent = git(repo, "rev-parse", "HEAD")
    (repo / "two").write_text("2", encoding="utf-8")
    commit(repo, "two", 1_700_000_100)
    git(repo, "checkout", "--detach")
    assert subprocess.run(
        ["git", "show-ref", "--verify", "--quiet", "refs/heads/main"], cwd=repo
    ).returncode != 0

    base = subprocess.check_output(
        ["python", str(METADATA), "resolve-base", "--repo", str(repo)], text=True
    ).strip()
    assert base == parent
