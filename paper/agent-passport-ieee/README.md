# Agent Passport — IEEE conference paper

IEEEtran (`conference`) source for *"Agent Passport: Compiled Sovereign Metadata
for Governed AI Agent Runtime Systems."* This is a derived, narrowed paper based
on the original ArgorixLang preprint; it introduces **no new experiments,
metrics, citations, or deployment claims**.

## Files

| File | Purpose |
|---|---|
| `main.tex` | Full paper (single file): body, four tables, three TikZ figure stubs. |
| `references.bib` | Citation keys reused verbatim from the original preprint. |
| `README.md` | This file. |

The figures are **schematic TikZ stubs** meant to compile and convey structure,
not final camera-ready diagrams. Refine them after the structure/length review.

## Requirements

A TeX distribution (TeX Live 2021+, MiKTeX, or MacTeX) providing:

- `IEEEtran` document class
- `tikz` with libraries `arrows.meta`, `positioning`, `fit`, `backgrounds`,
  `shapes.geometric`, `calc`
- `booktabs`, `listings`, `hyperref`, `cite`, `amsmath`, `amssymb`

These ship with a full TeX Live/MiKTeX install. On a minimal MiKTeX setup, enable
on-the-fly package installation so missing packages are fetched automatically.

## Compile

### Option A — latexmk (recommended)

```sh
latexmk -pdf main.tex
```

To clean intermediate files:

```sh
latexmk -c
```

### Option B — pdflatex + bibtex (manual passes)

```sh
pdflatex main
bibtex   main
pdflatex main
pdflatex main
```

The two trailing `pdflatex` passes resolve cross-references (`\ref`, `\cite`) and
the bibliography. Output: `main.pdf`.

## Target length

The paper is written to fit the IEEE conference target of **6–8 pages** in
two-column `IEEEtran` format (four tables, three figures). If it runs long after
edits, the first levers are the validation-rule listing in
Section IV and the Related Work prose.

## Claim-boundary note

The paper deliberately keeps strict boundaries: a passport is a **local
declaration**, not legal identity, DID/VC verification, credential issuance,
operational DNS, or authentication. Do not relax this language or add
deployment/compliance claims when editing — the taxonomy in Table IV (claim
boundaries) and the Limitations section are load-bearing.
