"""Resolve reproducible-build provenance without relying on local branch names."""

from __future__ import annotations

import argparse
import hashlib
import subprocess
from pathlib import Path


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", *args], cwd=repo, text=True, stderr=subprocess.DEVNULL
    ).strip()


def source_files(repo: Path) -> list[str]:
    tracked = git(repo, "ls-files").splitlines()
    roots = (
        "paper/sections/",
        "paper/appendices/",
        "paper/figures/",
        "paper/tables/",
        "paper/data/",
    )
    exact = {"paper/main.tex", "paper/references.bib"}
    excluded = {
        "paper/data/final-qa.json",
        "paper/argorixlang-preprint.pdf",
    }
    return [
        path
        for path in tracked
        if (path in exact or path.startswith(roots)) and path not in excluded
    ]


def stable_source_epoch(repo: Path) -> int:
    files = source_files(repo)
    if not files:
        raise RuntimeError("no tracked paper content inputs found")
    return int(git(repo, "log", "-1", "--format=%ct", "--", *files))


def resolve_base(repo: Path, explicit: str | None = None) -> str:
    if explicit:
        return git(repo, "rev-parse", "--verify", f"{explicit}^{{commit}}")
    try:
        git(repo, "show-ref", "--verify", "--quiet", "refs/remotes/origin/main")
    except subprocess.CalledProcessError:
        return git(repo, "rev-parse", "--verify", "HEAD^")
    return git(repo, "merge-base", "HEAD", "origin/main")


def input_manifest(repo: Path) -> tuple[str, int]:
    paper = repo / "paper"
    included = [
        paper / "main.tex",
        paper / "references.bib",
        paper / "requirements.txt",
        paper / "Makefile",
        *sorted((paper / "sections").glob("*.tex")),
        *sorted((paper / "appendices").glob("*.tex")),
        *sorted((paper / "scripts").glob("*.py")),
        *sorted((paper / "scripts").glob("*.ps1")),
        *sorted((paper / "figures").glob("*.pdf")),
        *sorted((paper / "tables").glob("*.tex")),
        *sorted((paper / "data").glob("*")),
    ]
    files = sorted(
        {path.resolve() for path in included if path.is_file()}
        - {(paper / "data/final-qa.json").resolve()}
    )
    digest = hashlib.sha256()
    for path in files:
        relative = path.relative_to(repo.resolve()).as_posix().encode()
        digest.update(relative + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest(), len(files)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("stable-epoch", "resolve-base", "manifest"))
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--base-ref")
    args = parser.parse_args()
    if args.command == "stable-epoch":
        print(stable_source_epoch(args.repo))
    elif args.command == "resolve-base":
        print(resolve_base(args.repo, args.base_ref))
    else:
        digest, count = input_manifest(args.repo)
        print(f"{digest} {count}")


if __name__ == "__main__":
    main()
