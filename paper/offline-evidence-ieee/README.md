# Offline-Verifiable Runtime Evidence Bundles — IEEE conference paper

IEEEtran (`conference`) source for *"Offline-Verifiable Runtime Evidence Bundles for
Governed AI Agent Systems."* This is a derived, narrowed paper based on the original
ArgorixLang preprint; it introduces **no new experiments, metrics, citations, or
deployment claims**. All empirical values are reused unchanged from the original
ArgorixLang corpus.

## Scope

This paper focuses on the **runtime evidence and offline verification mechanism**:
traces, SecurityReports, EvidenceBundles, semantic SHA-256 digests, the trace-event
ledger digest, canonical JSON, and offline verification. It is a companion to, and
deliberately distinct from, the *Agent Passport* paper (which concerns sovereign
metadata). See Section "Scope" in `main.tex`.

## Files

| File | Purpose |
|---|---|
| `main.tex` | Full paper (single file): body, five tables, Algorithm 1, three TikZ figures. |
| `references.bib` | Citation keys reused verbatim from the original preprint. |
| `README.md` | This file. |

The figures are **schematic TikZ stubs** meant to compile and convey structure, not
final camera-ready diagrams. Figure 1 is the evidence artifact graph; Figure 2 the
offline verification sequence; Figure 3 the two-axis integrity-vs-approval interpretation.

## Requirements

A TeX distribution (TeX Live 2021+, MiKTeX, or MacTeX) providing:

- `IEEEtran` document class
- `tikz` with libraries `arrows.meta`, `positioning`, `fit`, `backgrounds`,
  `shapes.geometric`, `calc`
- `booktabs`, `listings`, `hyperref`, `cite`, `amsmath`, `amssymb`

## Compile

### Option A — latexmk (recommended)

```sh
latexmk -pdf main.tex
```

### Option B — pdflatex + bibtex (manual passes)

```sh
pdflatex main
bibtex   main
pdflatex main
pdflatex main
```

Output: `main.pdf`.

## Target length

Written to fit the IEEE conference target of **6–8 pages** in two-column `IEEEtran`
format (five tables, Algorithm 1, three figures). If it runs long after edits, the first
levers are the Related Work prose and Section "Integrity Is Not Approval".

## Claim-boundary note

The paper deliberately keeps strict boundaries. Successful EvidenceBundle verification
means only **internal cross-artifact consistency relative to the bundle**. It does **not**
prove source integrity, producer authentication, absence of side effects, policy
approval, certification, identity/credential verification, immutability, post-quantum
security, or production isolation. The formal model (`Verify ⇏ PolicyApproved`),
Table "Integrity vs. approval", and the Limitations section are load-bearing — do not
relax this language or add compliance claims when editing.
