"""Resolve reproducible-build provenance without relying on local branch names."""

from __future__ import annotations

import argparse
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("stable-epoch", "resolve-base"))
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--base-ref")
    args = parser.parse_args()
    if args.command == "stable-epoch":
        print(stable_source_epoch(args.repo))
    else:
        print(resolve_base(args.repo, args.base_ref))


if __name__ == "__main__":
    main()
