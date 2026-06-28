# ArgorixLang arXiv Preprint Design Specification

**Date:** 2026-06-28  
**Status:** Approved editorial and methodological design  
**Target:** Academic English arXiv-style preprint, approximately 20 pages

## 1. Objective

Produce a complete, publication-oriented study of ArgorixLang as a compiled,
governed runtime for sovereign AI agents. The paper must connect language-level
contracts, fail-closed execution, agent passports, ATrust, DCP-AI, evidence
artifacts, and the proposed Sovereign DNS / Agent Naming Service direction.

The paper must be grounded in the current repository and in the runtime
artifacts under:

`demo/argorix-chatbot-runtime/generated/`

The study is an observational and reproducible systems study. It is not a
controlled security experiment, a certification, or proof of absolute safety.

## 2. Authors and affiliations

Author order:

1. Gustavo Venegas — Chilean Chamber of Artificial Intelligence
2. Edison Vazquez — Chilean Chamber of Artificial Intelligence
3. Danilo Naranjo — Ocular
4. Benjamin Gonzalez — Chilean Chamber of Artificial Intelligence

The LaTeX title page will distinguish lead authors and co-authors only if the
chosen arXiv class supports the distinction without inventing academic roles.
All four names will appear as authors.

## 3. Research framing

The primary framing is a systems paper with empirical evaluation. The paper
will answer:

1. How can governance, sovereign identity, and evidence requirements be
   represented as compiled language constructs?
2. How does the ArgorixLang pipeline constrain a chatbot request before a
   provider operation can be planned?
3. What evidence does the runtime generate, and how can its internal
   consistency be checked offline?
4. What do the existing runtime sessions demonstrate, and what do they not
   demonstrate?
5. How can Agent Passports, ATrust, DCP-AI, and future sovereign discovery fit
   into an open agentic-web architecture?

Project NANDA will be treated as foundational related work for open agent
discovery, identity, and trust. ATrust will be attributed to Edison Vazquez and
DCP-AI to Danilo Naranjo. Their relationship to ArgorixLang will be explained
without claiming that conceptual influence is equivalent to implemented
runtime behavior.

## 4. Claim boundaries

The paper may claim that the repository implements and emits:

- Argorix source, semantic validation, IR, bytecode, and VM execution paths;
- Agent Passport metadata, including jurisdiction, data residency, and
  sovereign `ans_name`;
- declarative ATrust identity, credential, handshake, trust-ledger, evidence
  mapping, bridge, policy, governance, hardening, and threat-model metadata;
- trace, SecurityReport, EvidenceBundle, and semantic SHA-256 digests;
- fail-closed planning and bounded adapter contracts in the demonstrated
  runtime.

The paper must not claim that the current system:

- resolves DNS, DIDs, or remote agent registries;
- authenticates real persons or agents;
- verifies real credentials or executes a cryptographic handshake;
- provides blockchain consensus or immutable storage;
- proves post-quantum security;
- certifies legal or regulatory compliance;
- guarantees security against all prompt injection or secret leakage;
- establishes production-grade external-provider isolation.

Sovereign DNS / Agent Naming Service resolution is architecture proposed for
future work. The currently implemented `ans_name` is a declarative passport
identifier, not an operational DNS resolver.

## 5. Dataset and methodology

The unit of analysis is one request directory under `generated/<requestId>/`.
The initial inventory contains 33 directories:

- 27 directories with all five expected artifacts;
- 6 directories containing only `session.argx`.

Expected artifacts:

1. `session.argx`
2. `session.argbc.json`
3. `session.trace.json`
4. `session.security.json`
5. `session.evidence.json`

The analysis pipeline will:

- inventory artifact completeness and byte size;
- parse all available JSON artifacts;
- classify execution, policy, provider-boundary, and review outcomes;
- count trace and ledger event families;
- inspect passport, ATrust, governance, hardening, and adapter metadata;
- validate digest format and cross-artifact references;
- run offline evidence verification where artifacts are complete;
- re-run representative compiler and VM checks where reproducible;
- preserve and report malformed, incomplete, contradictory, or unfavorable
  results;
- analyze authorized prompt text qualitatively while avoiding unnecessary
  disclosure of secrets or credentials.

The paper will explicitly examine cases where a top-level field such as
`security_checks` appears favorable while detailed policy results require
review or contain violations. Aggregate labels will never override the more
specific evidence.

Derived data will be stored in machine-readable CSV/JSON files. Scripts will
regenerate tables and figures from source artifacts.

## 6. Paper structure

1. Title, abstract, and keywords
2. Introduction and research questions
3. Background and related work
4. ArgorixLang language and compiler architecture
5. Sovereign identity: Passport, jurisdiction, residency, and naming
6. ATrust and DCP-AI trust architecture
7. Governed runtime and fail-closed execution
8. Evidence, reports, traces, and digest model
9. Dataset and experimental methodology
10. Quantitative and qualitative results
11. Security analysis and threat–mitigation mapping
12. Discussion and comparison with related work
13. Limitations and claim boundaries
14. Sovereign DNS and future work
15. Conclusion
16. References and technical appendices

The target is approximately 20 rendered pages including references, with
appendices allowed to extend the document when necessary for reproducibility.

## 7. Figure and table program

The paper will contain 10–12 vector figures selected from:

1. governed execution and sovereign trust stack;
2. compilation and evidence pipeline;
3. end-to-end request sequence diagram;
4. fail-closed decision state machine;
5. runtime outcome and artifact-completeness chart;
6. policy/control heat map;
7. evidence integrity and digest chain;
8. Passport–ATrust–governance relationship graph;
9. threat–mitigation matrix;
10. ArgorixLang evolution timeline;
11. implemented ANS versus proposed Sovereign DNS/DID discovery;
12. annotated artifact schema and claim-boundary plate.

The paper will contain 5–7 tables covering dataset inventory, language
constructs, runtime controls, empirical results, threat mappings, related work,
and explicit claim boundaries.

Figures must remain legible in print, use color-blind-safe palettes, include
captions that state the evidentiary meaning, and avoid decorative graphics that
do not communicate a result.

## 8. References

Bibliography sources will prioritize primary research papers, official
standards, project documentation, and repository artifacts. Required named
references include:

- Project NANDA;
- *Architecting the Internet of AI Agents*;
- ATrust — Agentic Trust Protocol;
- DCP-AI — Digital Citizenship Protocol for Autonomous AI.

The deliverable will include `references.bib`, using stable URLs, DOI/arXiv
identifiers where available, access dates for web-only material, and BibTeX
fields importable into Zotero. No bibliographic metadata will be invented when
a primary source does not provide it.

## 9. Deliverables

The `paper/` directory will contain:

- `main.tex`;
- modular section files;
- `references.bib`;
- `figures/` with vector source and rendered outputs;
- `data/` with derived CSV/JSON results;
- `scripts/` that regenerate analysis outputs;
- `appendices/`;
- `README.md` with build and reproduction commands;
- the compiled PDF when a reproducible LaTeX engine is available.

The build must fail visibly on missing required inputs. Generated results must
not silently fall back to fabricated example data.

## 10. Acceptance criteria

The work is complete when:

- the LaTeX source builds without errors;
- the PDF is approximately 20 pages and visually inspected;
- all runtime-derived values can be traced to generated artifacts;
- all figures and tables are reproducible;
- required named work is attributed;
- bibliography entries are validated against primary sources;
- unfavorable and incomplete observations are reported;
- implemented behavior, declarative metadata, and future work are clearly
  separated;
- no secret or credential value is published;
- the paper includes explicit limitations and reproducibility instructions.
