# ArgorixLang preprint

This directory contains the modular arXiv-style manuscript, normalized
observational data, generated vector figures and tables, and reproducibility
checks.

## Prerequisites

- Python 3.11 or newer
- PowerShell 7 or Windows PowerShell 5.1
- Rust and Cargo compatible with the repository toolchain
- GNU Make is optional; the canonical entrypoint is the PowerShell script

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
powershell -NoProfile -ExecutionPolicy Bypass -File paper/scripts/build_paper.ps1 -Target all
```

For auditability, the wrapper's equivalent stage commands are:

```powershell
python paper/scripts/analyze_runtime.py --input ../../demo/argorix-chatbot-runtime/generated --summary paper/data/runtime_summary.json --sessions paper/data/sessions.csv --events paper/data/event_counts.csv
powershell -File paper/scripts/verify_runtime.ps1 -InputRoot ../../demo/argorix-chatbot-runtime/generated -OutputPath paper/data/verification-results.json
python paper/scripts/render_tables.py --data paper/data --output paper/tables
python paper/scripts/generate_figures.py --data paper/data --output paper/figures
python paper/scripts/check_manuscript.py
python -m pytest paper/tests -q
```

Individual targets are `analyze`, `verify`, `tables`, `figures`, `test`,
`paper`, `qa`, and `clean`. `clean` removes only `paper/tmp`; it preserves the
input snapshot, normalized data, figures, tables, and final PDF. On systems
with GNU Make, the same targets are wrappers under `paper/Makefile`:

```powershell
make -C paper all
```

`verify_runtime.ps1` resolves Cargo in this order: an optional explicit
`-CargoPath`, `cargo` on `PATH`, then the legacy Windows Rustup location. For a
nonstandard installation:

```powershell
powershell -File paper/scripts/verify_runtime.ps1 -InputRoot ../../demo/argorix-chatbot-runtime/generated -OutputPath paper/data/verification-results.json -CargoPath D:/tools/cargo/bin/cargo.exe
```

## Reproducible LaTeX build

The build pins **Tectonic 0.16.9** and downloads the official Windows MSVC
release asset to `%LOCALAPPDATA%\ArgorixLang\tools\tectonic-0.16.9`, outside
the repository. The archive comes from the
[official GitHub release](https://github.com/tectonic-typesetting/tectonic/releases/tag/tectonic%400.16.9)
and must match GitHub's published SHA-256:
`131a24604785a9600989a3d91225f597df52ac06f00aeffe86fd529f99ee5cdd`.
No executable is committed.

Build and render the final artifact from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File paper/scripts/build_paper.ps1 -Target paper
powershell -NoProfile -ExecutionPolicy Bypass -File paper/scripts/build_paper.ps1 -Target qa
```

The stable output is `paper/argorixlang-preprint.pdf`; rendered QA pages are
temporary files under `paper/tmp/pdfs`. Tectonic stabilizes BibTeX citations
and cross-references automatically, while `SOURCE_DATE_EPOCH` is derived from
the input Git commit for stable PDF timestamps. The build fails on undefined
citations/references, bibliography errors, missing inputs, and any overfull
box. Harmless underfull boxes in narrow table cells, long bibliography URLs,
and BibTeX's empty-year warning for the deliberately undated unpublished
ATrust source are accepted and recorded in `paper/data/final-qa.json`. On this
Windows host, Tectonic also prints a harmless Fontconfig
default-configuration diagnostic; the bundled Latin Modern fonts remain
embedded in the PDF.

The pipeline intentionally preserves incomplete and unfavorable observations.
Successful offline EvidenceBundle verification is not policy approval, a
security proof, or certification.
