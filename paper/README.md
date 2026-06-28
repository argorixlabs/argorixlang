# ArgorixLang preprint

This directory contains the modular arXiv-style manuscript, normalized
observational data, generated vector figures and tables, and reproducibility
checks.

## Prerequisites

- Python 3.11 or newer
- PowerShell 7 or Windows PowerShell 5.1
- Rust and Cargo compatible with the repository toolchain
- A LaTeX distribution providing `pdflatex` and `bibtex` for the Task 6 PDF
  build

Install the pinned Python dependencies from the repository root:

```powershell
python -m pip install -r paper/requirements.txt
```

## Input snapshot

The analyzer treats the generated runtime corpus as an immutable input snapshot:
scripts write only to `paper/data`, `paper/tables`, and
`paper/figures`. Do not edit request directories during reproduction.

The corpus is ignored by Git. In this paper worktree, the tracked
`demo/argorix-chatbot-runtime/generated/.gitkeep` is not the corpus; the
authorized snapshot is in the primary checkout two levels above. The commands
below therefore use `../../demo/argorix-chatbot-runtime/generated`. In a normal
checkout where the snapshot is directly under the repository root, use
`demo/argorix-chatbot-runtime/generated` instead.

## Complete pipeline

Run from the repository root:

```powershell
python paper/scripts/analyze_runtime.py --input ../../demo/argorix-chatbot-runtime/generated --summary paper/data/runtime_summary.json --sessions paper/data/sessions.csv --events paper/data/event_counts.csv
powershell -File paper/scripts/verify_runtime.ps1 -InputRoot ../../demo/argorix-chatbot-runtime/generated -OutputPath paper/data/verification-results.json
python paper/scripts/render_tables.py --data paper/data --output paper/tables
python paper/scripts/generate_figures.py --data paper/data --output paper/figures
python paper/scripts/check_manuscript.py
python -m pytest paper/tests -q
```

`verify_runtime.ps1` resolves Cargo in this order: an optional explicit
`-CargoPath`, `cargo` on `PATH`, then the legacy Windows Rustup location. For a
nonstandard installation:

```powershell
powershell -File paper/scripts/verify_runtime.ps1 -InputRoot ../../demo/argorix-chatbot-runtime/generated -OutputPath paper/data/verification-results.json -CargoPath D:/tools/cargo/bin/cargo.exe
```

## LaTeX build

Task 6 will perform the authoritative PDF build and visual inspection. The
intended command sequence is:

```powershell
Set-Location paper
pdflatex -interaction=nonstopmode -halt-on-error main.tex
bibtex main
pdflatex -interaction=nonstopmode -halt-on-error main.tex
pdflatex -interaction=nonstopmode -halt-on-error main.tex
```

The pipeline intentionally preserves incomplete and unfavorable observations.
Successful offline EvidenceBundle verification is not policy approval, a
security proof, or certification.
