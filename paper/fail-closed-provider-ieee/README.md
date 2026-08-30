# Fail-Closed Provider Execution — IEEE conference paper

IEEEtran (`conference`) source for *"Fail-Closed Provider Execution for Governed AI Agent
Runtimes."* This is a derived, narrowed paper based on the original ArgorixLang preprint;
it introduces **no new experiments, metrics, citations, or deployment claims**. All
empirical values are reused unchanged from the original ArgorixLang corpus.

## Scope

This paper focuses on the **VM-level fail-closed provider boundary**: the separation of
declarative provider contracts from executable provider adapters, the executable provider
registry, capability/policy/runtime-profile checks before operation planning, and the
recorded negative controls (external execution blocked, network denied, secrets/key
material denied). It is a companion to, and deliberately distinct from, the *Agent
Passport* paper (sovereign metadata) and the *EvidenceBundle* paper (offline digest
verification). See Section "Scope" in `main.tex`.

## Files

| File | Purpose |
|---|---|
| `main.tex` | Full paper (single file): body, five tables, Algorithm 1, four TikZ figures. |
| `references.bib` | Citation keys reused verbatim from the original preprint. |
| `README.md` | This file. |

The figures are **schematic TikZ stubs** meant to compile and convey structure, not
final camera-ready diagrams. Figure 1 is the provider boundary architecture; Figure 2 the
declarative-vs-executable split; Figure 3 the fail-closed state machine; Figure 4 the
negative-control evidence.

## Tables

- Table I — Provider boundary concepts (Concept / Meaning / Implemented role / Not claimed)
- Table II — Fail-closed authorization predicates (Predicate / Definition / Interpretation)
- Table III — Observed runtime controls (Control-event / Observed value / Interpretation)
- Table IV — Threats and bounded mitigations (Threat / Bounded mitigation / Remaining gap)
- Table V — Declared vs. executable provider distinction (Statement / Supported? / Why)

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
format (five tables, Algorithm 1, four figures). If it runs long after edits, the first
levers are the Related Work prose and Section "Negative Controls".

## Claim-boundary note

The paper deliberately keeps strict boundaries. **Fail-closed** means only that, within
the implemented VM/application path, missing authorization, unsupported providers, denied
capabilities, or non-executable adapters terminate in **denial or review** rather than
falling through to execution. It does **not** prove OS-level sandboxing, container
isolation, host network containment, secret-store isolation, live-provider safety, adapter
correctness, absence of side effects outside instrumentation, compliance certification,
identity/credential verification, immutability, post-quantum security, or production
readiness. The formal planning rule (`Plan ⇒ MayExecute`, `Declared ⇏ Executable`),
Table V, and the Limitations section are load-bearing — do not relax this language or add
production-isolation claims when editing.
