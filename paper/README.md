# ArgorixLang preprint

This directory contains the modular arXiv-style manuscript, normalized
observational data, generated vector figures and tables, and reproducibility
checks.

## Validate

From the repository root:

```powershell
python paper/scripts/check_manuscript.py
python -m pytest paper/tests -q
```

The static manuscript check verifies modular inputs, the exact 12-figure and
7-table programs, labels and references, bibliography keys, claim-boundary
phrases, required quantitative claims, and basic TeX environment/brace syntax.

## Build

With a standard LaTeX distribution:

```powershell
Set-Location paper
pdflatex -interaction=nonstopmode -halt-on-error main.tex
bibtex main
pdflatex -interaction=nonstopmode -halt-on-error main.tex
pdflatex -interaction=nonstopmode -halt-on-error main.tex
```

The repository intentionally preserves incomplete and unfavorable observations.
Successful offline EvidenceBundle verification is not a policy approval,
security proof, or certification.
